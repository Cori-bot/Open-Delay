"""Reproducible synthetic loopback test; no screen capture or platform keys.

Requires a separately installed FFmpeg/ffprobe and a built open-delay-service.
Outputs remain in ignored .local/. Does not alter OBS or InstantDelay.
"""
import argparse
import collections
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
parser.add_argument('--mode', choices=['instant', 'overlay'], default='instant')
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
run = root / '.local/media-tests' / (args.mode + '-' + datetime.datetime.now().strftime('%Y%m%d-%H%M%S'))
run.mkdir(parents=True)
ffmpeg = str(args.ffmpeg.resolve())
ffprobe = str(args.ffmpeg.resolve().with_name('ffprobe.exe' if os.name == 'nt' else 'ffprobe'))
processes, logs = [], []
base, token = '', None
http = urllib.request.build_opener(urllib.request.ProxyHandler({}))
flags = subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0

def spawn(name, argv):
    log = (run / (name + '.log')).open('wb')
    logs.append(log)
    proc = subprocess.Popen(argv, stdout=log, stderr=log, stdin=subprocess.DEVNULL, creationflags=flags)
    processes.append(proc)
    return proc

def api(path, body=None):
    request = urllib.request.Request(base + '/api/v1' + path,
        data=None if body is None else json.dumps(body).encode(),
        headers={'Origin': base, 'Content-Type': 'application/json', **({'X-OpenDelay-Token': token} if token else {})})
    with http.open(request, timeout=5) as response:
        return json.load(response)

def packets(path):
    completed = subprocess.run([ffprobe, '-v', 'error', '-show_packets', '-show_data_hash', 'sha256',
        '-show_entries', 'packet=stream_index,dts_time,data_hash', '-of', 'json', str(path)],
        capture_output=True, check=True, creationflags=flags)
    return json.loads(completed.stdout)['packets']

def stream_types(path):
    completed=subprocess.run([ffprobe,'-v','error','-show_entries','stream=index,codec_type','-of','json',str(path)],capture_output=True,check=True,creationflags=flags)
    return {item['index']:item['codec_type'] for item in json.loads(completed.stdout)['streams']}

def analyze(reference, capture):
    source = collections.defaultdict(list)
    source_types=stream_types(reference)
    capture_types=stream_types(capture)
    for packet in packets(reference):
        source[(source_types[packet['stream_index']], packet['data_hash'])].append(float(packet['dts_time']))
    mappings = collections.defaultdict(list)
    unmatched = 0
    regressions = 0
    last = {}
    ambiguous = 0
    for packet in packets(capture):
        stream, dts = capture_types[packet['stream_index']], float(packet['dts_time'])
        if stream in last and dts < last[stream]:
            regressions += 1
        last[stream] = dts
        matches = source.get((stream, packet['data_hash']), [])
        if not matches:
            unmatched += 1
        elif len(matches) == 1:
            mappings[stream].append({'output': dts, 'source': matches[0]})
        else:
            ambiguous += 1
    jumps = []
    for previous, current in zip(mappings['video'], mappings['video'][1:]):
        delta = current['source'] - previous['source']
        if delta < -.3 or delta > .3:
            jumps.append({'output': current['output'], 'source_delta': round(delta, 3)})
    audio, video = mappings['audio'], mappings['video']
    audio_jumps=[{'output':b['output'],'source_delta':round(b['source']-a['source'],3)} for a,b in zip(audio,audio[1:]) if abs(b['source']-a['source'])>.3]
    cut_offsets=[min((abs(a['output']-v['output']) for a in audio_jumps if (a['source_delta']<0)==(v['source_delta']<0)),default=float('inf')) for v in jumps]
    vi, offsets = 0, []
    for a in audio:
        while vi + 1 < len(video) and abs(video[vi + 1]['output'] - a['output']) < abs(video[vi]['output'] - a['output']):
            vi += 1
        # A discontinuous source jump changes the mapping at the cut itself.
        # Compare stable intervals; measure inter-track cut timing separately.
        near_cut=any(abs(a['output']-jump['output'])<.1 for jump in jumps+audio_jumps)
        if video and not near_cut and abs(video[vi]['output'] - a['output']) <= .05:
            v = video[vi]
            offsets.append(abs((a['output'] - a['source']) - (v['output'] - v['source'])))
    decoded = subprocess.run([ffmpeg, '-v', 'error', '-i', str(capture), '-f', 'null', '-'], capture_output=True, creationflags=flags)
    (run / 'decode.log').write_bytes(decoded.stderr)
    result = {'unmatched_payloads': unmatched, 'dts_regressions': regressions,
        'ambiguous_payloads': ambiguous, 'unique_video': len(video), 'unique_audio': len(audio),
        'video_jumps': jumps, 'max_av_mapping_offset_s': max(offsets, default=None),
        'audio_jumps':audio_jumps,'max_av_cut_offset_s':max(cut_offsets,default=None),
        'decode_exit': decoded.returncode, 'decode_error_bytes': len(decoded.stderr)}
    (run / 'analysis.json').write_text(json.dumps(result, indent=2))
    (run / 'mappings.json').write_text(json.dumps(mappings))
    assert video and audio, 'No uniquely matched A/V packets'
    assert unmatched == 0 and regressions == 0 and decoded.returncode == 0 and not decoded.stderr, result
    assert any(j['source_delta'] < -2 for j in jumps), 'No replay observed'
    assert any(j['source_delta'] > 2 for j in jumps), 'No live-return observed'
    assert offsets and max(offsets) < .08, 'A/V source timeline alignment exceeds 80ms'
    assert cut_offsets and max(cut_offsets)<.08, 'A/V cut boundaries differ by more than 80ms'
    return result

try:
    fixture = run / 'reference.flv'
    # Video frame count is a visible source clock; chirps make audio payloads non-periodic.
    drawtext = "drawtext=fontfile='C\\:/Windows/Fonts/consola.ttf':text='%{n}':x=20:y=20:fontsize=32:fontcolor=white:box=1:boxcolor=black@0.7" if os.name == 'nt' else "drawtext=text='%{n}':x=20:y=20:fontsize=32:fontcolor=white"
    generated = subprocess.run([ffmpeg, '-hide_banner', '-loglevel', 'error', '-f', 'lavfi', '-i', 'testsrc2=size=640x360:rate=30',
        '-f', 'lavfi', '-i', 'aevalsrc=0.12*sin(2*PI*(260*t+13*t*t))|0.12*sin(2*PI*(390*t+19*t*t)):s=48000',
        '-t', '27', '-vf', drawtext, '-c:v', 'libx264', '-preset', 'ultrafast', '-b:v', '1500k', '-g', '30', '-bf', '0',
        '-c:a', 'aac', '-b:a', '128k', '-f', 'flv', str(fixture)], capture_output=True, creationflags=flags)
    (run / 'fixture.log').write_bytes(generated.stderr)
    assert generated.returncode == 0, generated.stderr.decode(errors='replace')
    service = spawn('service', [str(args.service.resolve()), '--data-dir', str(run / 'state'), '--api-port', '0', '--rtmp-port', '0'])
    runtime_path = run / 'state/runtime.json'
    deadline = time.monotonic() + 25
    while not runtime_path.exists():
        assert service.poll() is None, 'Service exited during startup'
        assert time.monotonic() < deadline, 'Service startup timed out'
        time.sleep(.1)
    runtime = json.loads(runtime_path.read_text())
    base = runtime['api_base']
    token = api('/session')['token']
    preferences = api('/preferences')
    preferences.update(mode=args.mode, buffer_ms=3000, disk_bytes=64*1024*1024, cache_bytes=8*1024*1024)
    # Running ring stays 2GiB for this launch; these settings only affect subsequent launches.
    api('/preferences', preferences)
    with socket.socket() as port_check:
        port_check.bind(('127.0.0.1', 0))
        sink_port = port_check.getsockname()[1]
    route = {'id':'custom','name':'Loopback research','platform':'custom','local_app':'test',
        'upstream_url':f'rtmp://127.0.0.1:{sink_port}/test','enabled':True,'include_in_group':True}
    api('/routes', route)
    sink = spawn('sink', [ffmpeg, '-hide_banner', '-loglevel', 'debug', '-listen', '1', '-i',
        f'rtmp://127.0.0.1:{sink_port}/test/test', '-map', '0', '-c', 'copy', '-f', 'flv', str(run / 'capture.flv')])
    time.sleep(.4)
    publisher = spawn('publisher', [ffmpeg, '-hide_banner', '-loglevel', 'warning', '-re', '-i', str(fixture),
        '-map', '0', '-c', 'copy', '-f', 'flv', f"rtmp://127.0.0.1:{runtime['rtmp_port']}/test/test"])
    start, done, events = time.monotonic(), set(), []
    schedule = [(8,'delay3',{'action':'set_delay','delay_ms':3000}), (13,'delay5',{'action':'set_delay','delay_ms':5000}),
        (21,'live',{'action':'go_live'})] if args.mode == 'instant' else [
        (5,'build3',{'action':'set_delay','delay_ms':3000}), (6,'cancel_build',{'action':'go_live'}),
        (11,'build3again',{'action':'set_delay','delay_ms':3000}), (21,'live',{'action':'go_live'})]
    while publisher.poll() is None and time.monotonic() - start < 40:
        elapsed = time.monotonic() - start
        status = api('/status')
        events.append({'elapsed':round(elapsed,3),'status':status})
        (run / 'events.json').write_text(json.dumps(events,indent=2))
        for at, name, command in schedule:
            if elapsed >= at and name not in done:
                response = api('/commands', dict(id=name,**command))
                events.append({'elapsed':round(elapsed,3),'command':name,'response':response})
                print(json.dumps({'command':name,'success':response['success'],'reason':response.get('reason')}),flush=True)
                assert response['success'], response
                done.add(name)
        time.sleep(.1)
    (run / 'events.json').write_text(json.dumps(events,indent=2))
    publisher.wait(timeout=5)
    sink.wait(timeout=15)
    assert publisher.returncode == 0 and sink.returncode == 0, 'Publisher/sink failed; inspect logs'
    final = api('/status')
    assert not final['is_connected'] and not final['transition_in_progress'], final
    result = analyze(fixture, run / 'capture.flv')
    print(json.dumps({'run':str(run),'result':result}),flush=True)
finally:
    if base and token:
        try:
            api('/shutdown', {})
        except Exception as error:
            print('Graceful shutdown request failed:', type(error).__name__, flush=True)
    for proc in reversed(processes):
        try:
            proc.wait(timeout=15)
        except subprocess.TimeoutExpired:
            proc.terminate()
            proc.wait(timeout=5)
    for log in logs:
        log.close()
