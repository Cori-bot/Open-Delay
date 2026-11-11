"""Build a Windows dependency inventory/SBOM and copy upstream license notices.

Run after cargo fetch and npm ci. Does not download sources or include research.
"""
from pathlib import Path
import json
import shutil
import subprocess
import tomllib
from urllib.parse import quote

root=Path(__file__).resolve().parents[1]
out=root/'dist/compliance'
notices=out/'licenses'
notices.mkdir(parents=True,exist_ok=True)
metadata=json.loads(subprocess.run(['cargo','metadata','--locked','--format-version','1','--filter-platform','x86_64-pc-windows-msvc'],cwd=root,capture_output=True,check=True).stdout)
packages={p['id']:p for p in metadata['packages']}
nodes={n['id']:n for n in metadata['resolve']['nodes']}
reachable=set()
pending=list(metadata['workspace_members'])
while pending:
    item=pending.pop()
    if item in reachable:continue
    reachable.add(item)
    for dep in nodes.get(item,{}).get('deps',[]):
        if any(kind['kind']!='dev' for kind in dep['dep_kinds']):pending.append(dep['pkg'])
lock=tomllib.loads((root/'Cargo.lock').read_text())
checksums={(p['name'],p['version']):p.get('checksum') for p in lock['package']}
components=[]
inventory=[]
missing=[]

def add(name,version,license_id,ecosystem,directory,checksum=None):
    if license_id:license_id=' '.join(license_id.replace('/',' OR ').split())
    slug=(ecosystem+'-'+name+'-'+version).replace('/','_').replace('@','')
    copied=[]
    for path in directory.iterdir():
        if path.is_file() and path.name.lower().startswith(('license','licence','copying','notice','copyright')) and path.stat().st_size<1024*1024:
            target=notices/slug/path.name
            target.parent.mkdir(exist_ok=True,parents=True)
            shutil.copyfile(path,target)
            copied.append(target.relative_to(out).as_posix())
    if not license_id:missing.append(f'{name}@{version}: license expression missing')
    component={'type':'library','name':name,'version':version,'purl':f'pkg:{ecosystem}/{quote(name,safe="/")}@{version}'}
    source=f'https://crates.io/crates/{name}/{version}' if ecosystem=='cargo' else f'https://www.npmjs.com/package/{name}/v/{version}'
    component['externalReferences']=[{'type':'distribution','url':source}]
    if license_id:component['licenses']=[{'expression':license_id}]
    if checksum:component['hashes']=[{'alg':'SHA-256','content':checksum}]
    components.append(component)
    inventory.append({'name':name,'version':version,'ecosystem':ecosystem,'license':license_id,'notices':copied})

for key in sorted(reachable):
    package=packages[key]
    if not package['source']:continue
    add(package['name'],package['version'],package.get('license'),'cargo',Path(package['manifest_path']).parent,checksums.get((package['name'],package['version'])))

for relative in ['ui']:
    base=root/relative
    lock=json.loads((base/'package-lock.json').read_text())
    for name,item in sorted(lock['packages'].items()):
        if not name or item.get('dev') or item.get('optional'):continue
        directory=base/name
        manifest=json.loads((directory/'package.json').read_text(encoding='utf-8'))
        add(manifest['name'],manifest['version'],manifest.get('license'),'npm',directory)

bom={'bomFormat':'CycloneDX','specVersion':'1.5','version':1,'metadata':{'component':{'type':'application','name':'Open-Delay','version':'0.1.0'}},'components':components}
(out/'bom.json').write_text(json.dumps(bom,indent=2),encoding='utf-8')
(out/'inventory.json').write_text(json.dumps(inventory,indent=2),encoding='utf-8')
(out/'README.txt').write_text('Open-Delay third-party inventory. Includes normal/build dependencies reachable for Windows, not development-only dependencies. Components retain their own licenses. Complete license files copied when present in published source packages. Original source packages can be obtained using the distribution links in bom.json; use the pinned versions listed in inventory.json. No proprietary research assets are included.\n',encoding='utf-8')
print(json.dumps({'components':len(components),'copied_notices':sum(len(p['notices']) for p in inventory),'missing_license_expressions':missing,'output':str(out)}))
if missing:raise SystemExit(1)
