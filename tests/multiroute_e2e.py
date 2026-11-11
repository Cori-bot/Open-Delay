"""Two-route RTMP isolation and upstream-recovery test, entirely on loopback.

Requires a built service and external FFmpeg/ffprobe. All generated files live
under ignored .local/. No OBS settings, native windows or platform keys are used.
"""
import argparse
import datetime
import json
import os
from pathlib import Path
import socket
import subprocess
import time
import urllib.request


parser = argparse.ArgumentParser()
parser.add_argument('--ffmpeg', required=True, type=Path)
parser.add_argument('--service', type=Path, default=Path('target/debug/open-delay-service.exe'))
parser.add_argument('--reference', type=Path)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
run = root / '.local/multiroute-tests' / datetime.datetime.now().strftime('%Y%m%d-%H%M%S')
run.mkdir(parents=True)
ffmpeg = str(args.ffmpeg.resolve())
ffprobe = str(args.ffmpeg.resolve().with_name('ffprobe.exe' if os.name == 'nt' else 'ffprobe'))
flags = subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0
http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
owned, log_files = [], []
base, token = '', None
clock = time.monotonic()


def record(kind, **fields):
    event = {'elapsed_s': round(time.monotonic() - clock, 3), 'kind': kind, **fields}
    with (run / 'events.jsonl').open('a', encoding='utf-8') as stream:
        stream.write(json.dumps(event) + '\n')
    if kind != 'status':
        print(json.dumps(event), flush=True)


def spawn(name, argv):
    log = (run / (name + '.log')).open('wb')
    log_files.append(log)
    proc = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=log, stderr=log, creationflags=flags)
    owned.append((name, proc))
    record('process', name=name, pid=proc.pid)
    return proc


def api(path, body=None):
    request = urllib.request.Request(base + '/api/v1' + path,
        data=None if body is None else json.dumps(body).encode(),
        headers={'Origin': base, 'Content-Type': 'application/json',
            **({'X-OpenDelay-Token': token} if token else {})})
    with http.open(request, timeout=5) as response:
        return json.load(response)


def statuses():
    status = api('/status')
    record('status', status=status)
    return status, {route['id']: route for route in status['routes']}


def wait_for(label, predicate, timeout=12):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        last = statuses()
        if predicate(*last):
            record('verified', condition=label)
            return last
        time.sleep(.1)
    raise AssertionError(f'{label} timed out; last routes: {last[1] if last else None}')


def observe(seconds):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        statuses()
        time.sleep(.1)


def command(name, action, **fields):
    response = api('/commands', {'id': name, 'action': action, **fields})
    record('command', name=name, response=response)
    assert response['success'], response
    return response


def free_port():
    with socket.socket() as check:
        check.bind(('127.0.0.1', 0))
        return check.getsockname()[1]


def sink(name, port, capture):
    return spawn(name, [ffmpeg, '-hide_banner', '-loglevel', 'warning', '-listen', '1', '-i',
        f'rtmp://127.0.0.1:{port}/test/key', '-map', '0', '-c', 'copy', '-f', 'flv', str(capture)])


def publisher(name, fixture, app, port):
    return spawn(name, [ffmpeg, '-hide_banner', '-loglevel', 'warning', '-stream_loop', '-1', '-re', '-i',
        str(fixture), '-t', '30', '-map', '0', '-c', 'copy', '-f', 'flv',
        f'rtmp://127.0.0.1:{port}/{app}/key'])


def packet_data(path, video_only=False):
    argv = [ffprobe, '-v', 'error']
    if video_only:
        argv += ['-select_streams', 'v:0']
    argv += ['-show_packets', '-show_data_hash', 'sha256', '-show_entries',
        'packet=stream_index,dts_time,data_hash', '-of', 'json', str(path)]
    result = subprocess.run(argv, capture_output=True, check=True, creationflags=flags)
    return json.loads(result.stdout)['packets']


def analyze_capture(path, reference, other_reference):
    packets = packet_data(path)
    video = packet_data(path, True)
    own_hashes = {packet['data_hash'] for packet in packet_data(reference, True)}
    other_hashes = {packet['data_hash'] for packet in packet_data(other_reference, True)}
    mismatches = sum(packet['data_hash'] not in own_hashes for packet in video)
    foreign = sum(packet['data_hash'] in other_hashes for packet in video)
    regressions, previous = 0, {}
    for packet in packets:
        stream, dts = packet['stream_index'], float(packet['dts_time'])
        if stream in previous and dts < previous[stream]:
            regressions += 1
        previous[stream] = dts
    decode = subprocess.run([ffmpeg, '-v', 'error', '-i', str(path), '-f', 'null', '-'],
        capture_output=True, creationflags=flags)
    path.with_suffix('.decode.log').write_bytes(decode.stderr)
    result = {'capture': path.name, 'packets': len(packets), 'video_packets': len(video),
        'video_payload_mismatches': mismatches, 'other_route_video_payloads': foreign,
        'dts_regressions': regressions, 'decode_exit': decode.returncode,
        'decode_error_bytes': len(decode.stderr)}
    assert video and mismatches == 0 and foreign == 0 and regressions == 0, result
    assert decode.returncode == 0 and not decode.stderr, result
    return result


try:
    source = args.reference
    if source is None:
        references = sorted((root / '.local/media-tests').glob('*/reference.flv'))
        assert references, 'Pass --reference or run tests/media_e2e.py first'
        source = references[-1]
    source = source.resolve()
    assert source.is_file(), 'Reference fixture is missing'
    # Route B has different encoded video. Hash checks can therefore detect
    # accidental fanout or session mixing instead of only counting packets.
    reference_b = run / 'reference-b.flv'
    generated = subprocess.run([ffmpeg, '-hide_banner', '-loglevel', 'error', '-i', str(source),
        '-vf', 'hue=h=120', '-c:v', 'libx264', '-preset', 'ultrafast', '-b:v', '1500k',
        '-g', '30', '-bf', '0', '-c:a', 'copy', '-f', 'flv', str(reference_b)],
        capture_output=True, creationflags=flags)
    (run / 'fixture-b.log').write_bytes(generated.stderr)
    assert generated.returncode == 0, 'Route B fixture generation failed'
    port_a, port_b = free_port(), free_port()
    while port_a == port_b:
        port_b = free_port()
    state = run / 'state'
    state.mkdir()
    config = {'preferences': {'mode': 'instant', 'buffer_ms': 2000, 'disk_bytes': 64 * 1024 * 1024,
        'cache_bytes': 8 * 1024 * 1024, 'api_port': 0, 'rtmp_port': 0, 'onboarding_complete': True},
        'routes': [
            {'id': 'twitch', 'name': 'Inactive grouped route', 'platform': 'twitch', 'local_app': 'inactive',
                'upstream_url': 'rtmp://127.0.0.1:1/inactive', 'enabled': False, 'include_in_group': True},
            {'id': 'custom', 'name': 'Route A', 'platform': 'custom', 'local_app': 'route_a',
                'upstream_url': f'rtmp://127.0.0.1:{port_a}/test', 'enabled': True, 'include_in_group': True},
            {'id': 'kick', 'name': 'Route B', 'platform': 'custom', 'local_app': 'route_b',
                'upstream_url': f'rtmp://127.0.0.1:{port_b}/test', 'enabled': True, 'include_in_group': True}]}
    (state / 'config.json').write_text(json.dumps(config))
    service = spawn('service', [str(args.service.resolve()), '--data-dir', str(state)])
    deadline = time.monotonic() + 20
    while not (state / 'runtime.json').exists():
        assert service.poll() is None, 'Service exited during startup'
        assert time.monotonic() < deadline, 'Service startup timed out'
        time.sleep(.1)
    runtime = json.loads((state / 'runtime.json').read_text())
    base = runtime['api_base']
    token = api('/session')['token']
    sink_a = sink('sink-a-before-drop', port_a, run / 'capture-a-before-drop.flv')
    sink_b = sink('sink-b', port_b, run / 'capture-b.flv')
    time.sleep(.4)
    publisher_a = publisher('publisher-a', source, 'route_a', runtime['rtmp_port'])
    publisher_b = publisher('publisher-b', reference_b, 'route_b', runtime['rtmp_port'])
    _, routes = wait_for('two routes ready', lambda _, r: all(r[key]['bytes_sent'] > 0 and r[key]['buffer']['fill_ms'] >= 2000 for key in ('custom', 'kick')))
    generation_a, generation_b = routes['custom']['generation'], routes['kick']['generation']
    response = command('group-delay-first', 'set_delay', delay_ms=2000)
    assert {item['route_id'] for item in response['results']} == {'custom', 'kick'}
    wait_for('both routes delayed', lambda s, r: not s['transition_in_progress'] and all(r[key]['effective_delay_ms'] >= 1800 for key in ('custom', 'kick')))
    observe(2)
    _, before = statuses()
    sink_a.terminate()
    sink_a.wait(timeout=5)
    record('injected_failure', target='sink-a', exit=sink_a.returncode)
    wait_for('route A recovering', lambda _, r: r['custom']['transport_state'] == 'recovering')
    observe(3)
    _, during = statuses()
    assert during['kick']['bytes_sent'] > before['kick']['bytes_sent'] + 100_000
    assert during['kick']['is_connected'] and during['kick']['generation'] == generation_b
    assert during['kick']['effective_delay_ms'] >= 1800 and not during['kick']['buffer']['degraded']
    record('verified', condition='route B continues delayed while route A upstream is absent')
    capture_a_after = run / 'capture-a-after-reconnect.flv'
    sink_a_after = sink('sink-a-after-reconnect', port_a, capture_a_after)
    wait_for('route A reconnects without replacing publisher', lambda _, r:
        r['custom']['transport_state'] == 'connected' and r['custom']['bytes_sent'] > during['custom']['bytes_sent'] + 30_000)
    _, recovered = statuses()
    assert recovered['custom']['generation'] == generation_a
    assert recovered['custom']['desired_delay_ms'] == 2000
    assert recovered['custom']['effective_delay_ms'] >= 1800
    observe(2)
    _, stable_recovery = statuses()
    assert stable_recovery['custom']['transport_state'] == 'connected'
    assert stable_recovery['custom']['generation'] == generation_a
    assert stable_recovery['custom']['desired_delay_ms'] == 2000
    assert stable_recovery['custom']['effective_delay_ms'] >= 1800
    assert stable_recovery['custom']['bytes_sent'] > recovered['custom']['bytes_sent'] + 100_000
    record('verified', condition='reconnected route sustains delayed output for two seconds')
    command('group-live', 'go_live')
    wait_for('both routes return live', lambda s, r: not s['transition_in_progress'] and all(r[key]['effective_delay_ms'] == 0 for key in ('custom', 'kick')))
    command('group-delay-second', 'set_delay', delay_ms=2000)
    wait_for('both routes delayed again', lambda s, r: not s['transition_in_progress'] and all(r[key]['effective_delay_ms'] >= 1800 for key in ('custom', 'kick')))
    publisher_a.terminate()
    publisher_a.wait(timeout=5)
    record('injected_failure', target='publisher-a', exit=publisher_a.returncode)
    wait_for('route A disconnected while route B remains connected', lambda _, r: not r['custom']['is_connected'] and r['kick']['is_connected'])
    response = command('survivor-live', 'go_live')
    assert [item['route_id'] for item in response['results']] == ['kick'], response
    wait_for('inactive grouped routes do not block survivor return live', lambda s, r: not s['transition_in_progress'] and r['kick']['effective_delay_ms'] == 0)
    observe(2)
    publisher_b.wait(timeout=30)
    assert publisher_b.returncode == 0, 'Route B publisher failed without an injected fault'
    sink_b.wait(timeout=12)
    sink_a_after.wait(timeout=12)
    assert sink_b.returncode == 0 and sink_a_after.returncode == 0, 'A complete recording sink failed'
    final, _ = wait_for('all sessions disconnected cleanly', lambda s, _: not s['is_connected'] and not s['transition_in_progress'])
    results = [analyze_capture(run / 'capture-b.flv', reference_b, source),
        analyze_capture(capture_a_after, source, reference_b)]
    summary = {'run': str(run), 'captures': results, 'upstream_recovery': True,
        'inactive_route_does_not_block': True, 'final_connected': final['is_connected']}
    (run / 'summary.json').write_text(json.dumps(summary, indent=2))
    record('passed', **summary)
finally:
    if base and token:
        try:
            api('/shutdown', {})
        except Exception as error:
            record('cleanup_warning', action='graceful_shutdown', error_type=type(error).__name__)
    for name, proc in reversed(owned):
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=5)
        record('process_exit', name=name, code=proc.returncode)
    for log in log_files:
        log.close()
