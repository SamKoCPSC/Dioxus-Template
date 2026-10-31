#!/usr/bin/env python3
"""Run Dioxus on the host with isolated Docker PostgreSQL/mock OAuth. No secret output."""
import argparse,hashlib,json,os,re,secrets,shutil,signal,socket,subprocess,sys,tempfile,time,urllib.request,urllib.error
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
POSTGRES='postgres:17-bookworm@sha256:84560e3b9c6874893fc4e2854f5dc3e7c1a37bc9d1dfd7a8c641310ae22ba5ad'
BASE_STATE=Path(os.environ.get('APP_HOST_STATE_ROOT',str(Path(tempfile.gettempdir())/'app-host-dev'))).resolve()

def normalized(name):
    if not re.fullmatch(r'[a-z0-9][a-z0-9-]{0,39}',name):raise ValueError('Use a lowercase instance name of up to 40 letters/digits/hyphens')
    return 'app-host-'+hashlib.sha256(str(ROOT).encode()).hexdigest()[:8]+'-'+name

def private_write(path,text):
    fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_TRUNC,0o600)
    with os.fdopen(fd,'w') as file:file.write(text)
    path.chmod(0o600)

def state_path(name):
    if BASE_STATE==ROOT or ROOT in BASE_STATE.parents:raise ValueError('APP_HOST_STATE_ROOT must be outside the source checkout')
    return BASE_STATE/hashlib.sha256(str(ROOT).encode()).hexdigest()[:12]/normalized(name)

def command_env(meta,state,managed):
    env=os.environ.copy()
    if managed:
        for key in ['DOCKER_CONTEXT','DOCKER_TLS','DOCKER_TLS_VERIFY','DOCKER_CERT_PATH']:env.pop(key,None)
        env['DOCKER_HOST']='unix:///var/run/docker.sock'
    env.update(APP_INSTANCE_ID=meta['instance'],APP_PROJECT_NAME=meta['instance'],APP_INSTANCE_TOKEN=meta['token'],APP_WORKTREE=str(ROOT),APP_RUNTIME_DIR=str(state))
    return env

def compose(meta,state,managed,*args,**kwargs):
    return subprocess.run(['docker','compose','--project-name',meta['instance'],'--file',str(ROOT/'docker-compose.yml'),'--file',str(state/'host.override.yml'),*args],env=command_env(meta,state,managed),check=True,**kwargs)

def port(meta,state,managed,service,internal):
    result=compose(meta,state,managed,'port',service,str(internal),capture_output=True,text=True)
    value=int(result.stdout.strip().rsplit(':',1)[1])
    if not 1<=value<=65535:raise ValueError('Invalid assigned loopback port')
    return value

def save_meta(state,meta):private_write(state/'metadata.json',json.dumps(meta,indent=2)+'\n')

def load_meta(state):
    meta=json.loads((state/'metadata.json').read_text())
    if meta.get('worktree')!=str(ROOT) or meta.get('instance')!=state.name:raise ValueError('Instance ownership/worktree mismatch')
    return meta

def start_signature(pid):
    state=subprocess.run(['ps','-p',str(pid),'-o','stat='],capture_output=True,text=True)
    if state.stdout.strip().startswith('Z'):return ''
    result=subprocess.run(['ps','-ww','-p',str(pid),'-o','lstart=','-o','args='],capture_output=True,text=True)
    return result.stdout.strip() if result.returncode==0 else ''

def cleanup(meta,state,managed):
    pid=meta.get('pid')
    if pid and start_signature(pid):
        if start_signature(pid)!=meta.get('process_signature'):raise ValueError('Refusing to signal a PID whose identity changed')
        os.killpg(pid,signal.SIGTERM)
        for _ in range(40):
            if not start_signature(pid):break
            time.sleep(.25)
    env=command_env(meta,state,managed)
    ids=subprocess.run(['docker','ps','-aq','--filter','label=com.app.dev-instance='+meta['instance']],env=env,capture_output=True,text=True,check=True).stdout.split()
    for container in ids:
        labels=json.loads(subprocess.run(['docker','inspect','--format','{{json .Config.Labels}}',container],env=env,capture_output=True,text=True,check=True).stdout)
        if labels.get('com.app.dev-token')!=meta['token'] or labels.get('com.app.dev-instance')!=meta['instance']:raise ValueError('Refusing to remove resources with mismatched ownership labels')
    compose(meta,state,managed,'down','--volumes','--remove-orphans',stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)

def up(args,state):
    dx=args.dx or shutil.which('dx')
    if not dx:raise ValueError('Install dioxus-cli@0.7.9 --locked, or provide --dx PATH, before starting host mode')
    if '0.7.9' not in subprocess.check_output([dx,'--version'],text=True):raise ValueError('Host mode requires Dioxus CLI 0.7.9')
    # Per-worktree lock, independent of the user-selected instance name.
    state.parent.mkdir(parents=True,exist_ok=True,mode=0o700)
    lock=state.parent/'worktree.lock'
    try:
        descriptor=os.open(lock,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)
    except FileExistsError:raise ValueError('This worktree already has a host instance; inspect it and use its exact down command')
    with os.fdopen(descriptor,'w') as file:file.write(state.name+'\n')
    if state.exists():lock.unlink();raise ValueError('Instance state already exists; inspect it and run its exact down command')
    state.mkdir(mode=0o700)
    meta={'instance':state.name,'worktree':str(ROOT),'token':secrets.token_hex(16),'state':'starting','pid':None,'managed':args.managed,'created':False}
    save_meta(state,meta)
    private_write(state/'instance.env','APP_ENV=local\n')
    private_write(state/'host.override.yml',f'''services:
  postgres:
    image: {POSTGRES}
  mock-github-api:
    ports:
      - target: 8080
        published: 0
        host_ip: 127.0.0.1
        protocol: tcp
    healthcheck:
      test: [CMD-SHELL, "wget -Y off -qO- http://127.0.0.1:8080/isalive || exit 1"]
  mock-oauth:
    healthcheck:
      test: [CMD-SHELL, "wget -Y off -qO- http://127.0.0.1:8080/isalive || exit 1"]
''')
    try:
        env=command_env(meta,state,args.managed)
        for kind in ['container','network','volume']:
            command=['docker','ps','-aq'] if kind=='container' else ['docker',kind,'ls','-q']
            found=subprocess.run(command+['--filter','label=com.docker.compose.project='+meta['instance']],env=env,capture_output=True,text=True,check=True).stdout.strip()
            if found:raise ValueError('Docker instance name already exists; choose another name and preserve its resources')
        meta['created']=True
        with (state/'infrastructure.log').open('w') as log:
            compose(meta,state,args.managed,'up','--detach','--wait','--wait-timeout','180','--no-build','postgres','mock-oauth','mock-github-api',stdout=log,stderr=subprocess.STDOUT)
        database_port=port(meta,state,args.managed,'postgres',5432)
        oauth_port=port(meta,state,args.managed,'mock-oauth',8080)
        github_port=port(meta,state,args.managed,'mock-github-api',8080)
        with socket.socket() as sock:sock.bind(('127.0.0.1',0));app_port=sock.getsockname()[1]
        origin=f'http://127.0.0.1:{app_port}';oauth=f'http://127.0.0.1:{oauth_port}';github=f'http://127.0.0.1:{github_port}'
        cookie='app_session_'+meta['instance'].replace('-','_')
        config={'APP_ENV':'local','BASE_URL':origin,'DATABASE_URL':f'postgres://app:app_local_pass@127.0.0.1:{database_port}/app','SESSION_SECRET':secrets.token_hex(32),'APP_SESSION_COOKIE_NAME':cookie,'STORAGE_ENABLED':'false',
                'GOOGLE_CLIENT_ID':'google','GOOGLE_CLIENT_SECRET':'secret','GOOGLE_AUTH_URL':oauth+'/google/authorize','GOOGLE_TOKEN_URL':oauth+'/google/token','GOOGLE_USERINFO_URL':oauth+'/google/userinfo',
                'GITHUB_CLIENT_ID':'github','GITHUB_CLIENT_SECRET':'secret','GITHUB_OAUTH_EXTRA_SCOPES':'openid','GITHUB_AUTH_URL':oauth+'/github/authorize','GITHUB_TOKEN_URL':oauth+'/github/token','GITHUB_USERINFO_URL':github+'/github/userinfo','GITHUB_EMAILS_URL':github+'/github/emails'}
        private_write(state/'instance.env',''.join(f'{key}={value}\n' for key,value in config.items()))
        with (state/'schema.log').open('w') as log:
            compose(meta,state,args.managed,'exec','-T','postgres','psql','-U','app','-d','app','--set','ON_ERROR_STOP=1',input=(ROOT/'migrations/schema.sql').read_text(),text=True,stdout=log,stderr=subprocess.STDOUT)
        env=os.environ.copy();env.update(config)
        env.setdefault('CARGO_TARGET_DIR',str(ROOT/'target'));env.setdefault('CARGO_BUILD_JOBS','4')
        cache=state/'session';cache.mkdir()
        with (state/'app.log').open('w') as log:
            process=subprocess.Popen([dx,'serve','--platform','web','--addr','127.0.0.1','--port',str(app_port),'--open','false','--interactive','false','--force-sequential','true','--debug-symbols','false','--session-cache-dir',str(cache)],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
        meta.update(pid=process.pid,process_signature=start_signature(process.pid),app_url=origin,cookie_name=cookie,database_port=database_port);save_meta(state,meta)
        print('Infrastructure ready; compiling the host application. Logs:',state/'app.log',flush=True)
        opener=urllib.request.build_opener(urllib.request.ProxyHandler({})) # Only the proven loopback origin above.
        for _ in range(args.timeout):
            if process.poll() is not None:raise ValueError('Dioxus stopped before readiness; inspect the private app log')
            try:
                with opener.open(origin+'/healthz',timeout=2) as response:
                    if response.status==200:break
            except (urllib.error.URLError,TimeoutError):pass
            time.sleep(1)
        else:raise ValueError('Application readiness timed out; inspect the private app log')
        meta['state']='ready';save_meta(state,meta)
        print('Application:',origin);print('Host instance:',args.instance);print('State:',state)
        print('Stop with: python3 scripts/host-dev.py down '+args.instance)
    except BaseException:
        try:
            owned=(state/'metadata.json').exists() and load_meta(state).get('token')==meta['token']
            if owned and meta.get('created'):cleanup(meta,state,args.managed)
        finally:
            if (state/'metadata.json').exists() and load_meta(state).get('token')==meta['token']:
                meta['state']='failed';save_meta(state,meta)
            if lock.exists() and lock.read_text().strip()==state.name:lock.unlink()
        raise

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation',choices=['up','down','status']);parser.add_argument('instance')
    parser.add_argument('--dx');parser.add_argument('--managed',action='store_true');parser.add_argument('--timeout',type=int,default=1200)
    args=parser.parse_args()
    try:
        state=state_path(args.instance)
        if args.operation=='up':up(args,state)
        elif args.operation=='status':
            meta=load_meta(state);print(json.dumps({key:meta.get(key) for key in ['instance','state','app_url','pid']},indent=2))
        else:
            meta=load_meta(state)
            if meta.get('created'):cleanup(meta,state,meta.get('managed',args.managed))
            shutil.rmtree(state)
            lock=state.parent/'worktree.lock'
            if lock.exists() and lock.read_text().strip()==state.name:lock.unlink()
            print('Removed only host instance:',args.instance)
    except (ValueError,OSError,subprocess.SubprocessError) as error:
        # Subprocess errors can contain command details; report their type, not env/headers.
        message=str(error) if isinstance(error,ValueError) else type(error).__name__
        parser.exit(1,'Host development failed: '+message+'\n')
if __name__=='__main__':main()
