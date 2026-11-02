#!/usr/bin/env python3
"""Exercise a ready local template instance using synthetic OAuth. Never writes auth state."""
import argparse,asyncio,importlib.util,json,os,re,shutil,sys,tempfile,time
from pathlib import Path
from urllib.parse import urlsplit,parse_qs,quote
ROOT=Path(__file__).resolve().parents[1]

def instance_metadata(args):
    if args.host:
        spec=importlib.util.spec_from_file_location('host_dev',ROOT/'scripts/host-dev.py')
        host=importlib.util.module_from_spec(spec);spec.loader.exec_module(host)
        meta=host.load_meta(host.state_path(args.instance))
        return meta['state'],meta['app_url'],meta['cookie_name']
    name=args.instance if args.instance.startswith('app-') else 'app-'+args.instance
    if not re.fullmatch(r'app-[a-z0-9-]+',name):raise ValueError('Invalid instance name')
    path=Path(os.environ.get('APP_RUNTIME_ROOT',str(Path(tempfile.gettempdir())/'app-dev')))/name/'metadata'
    values=dict(line.split('=',1) for line in path.read_text().splitlines() if '=' in line)
    return values['STATE'],values['APP_URL'],values['SESSION_COOKIE_NAME']

async def exercise_drawer(page,output,prefix,signed_in):
    from playwright.async_api import expect
    trigger=page.get_by_role('button',name='Open navigation menu',exact=True)
    dialog=page.get_by_role('dialog',name='Navigation',exact=True)
    close=dialog.get_by_role('button',name='Close drawer',exact=True)
    original=await page.evaluate("[document.documentElement,document.body].map(e=>[e.style.getPropertyValue('overflow'),e.style.getPropertyPriority('overflow')])")
    await trigger.click()
    await expect(dialog).to_be_visible()
    await expect(trigger).to_have_attribute('aria-expanded','true')
    await expect(close).to_be_focused()
    assert await page.evaluate("[document.documentElement,document.body].every(e=>getComputedStyle(e).overflow==='hidden')")
    await dialog.get_by_role('heading',name='Navigation',exact=True).click()
    await expect(dialog).to_be_visible()
    # Explicit wrap in both directions and native background inertness.
    await close.focus()
    await page.keyboard.press('Shift+Tab')
    await expect(dialog.get_by_role('button',name='Toggle navigation theme',exact=True)).to_be_focused()
    await page.keyboard.press('Tab')
    await expect(close).to_be_focused()
    assert await page.evaluate("(()=>{document.querySelector('.brand').focus();return document.querySelector('dialog[open]').contains(document.activeElement)})()")
    await expect(dialog.get_by_role('link',name='Dashboard',exact=True)).to_have_count(1 if signed_in else 0)
    await expect(dialog.get_by_role('link',name='Sign in',exact=True)).to_have_count(0 if signed_in else 1)
    assert not await page.evaluate('document.documentElement.scrollWidth>innerWidth')
    await page.screenshot(path=str(output/(prefix+'-drawer.png')),full_page=False)
    await page.keyboard.press('Escape')
    await expect(dialog).to_have_count(0)
    await expect(trigger).to_be_focused()
    await expect(trigger).to_have_attribute('aria-expanded','false')
    assert await page.evaluate("[document.documentElement,document.body].map(e=>[e.style.getPropertyValue('overflow'),e.style.getPropertyPriority('overflow')])")==original
    await trigger.click()
    bounds=await dialog.bounding_box()
    viewport=page.viewport_size
    await page.mouse.click(min(viewport['width']-2,bounds['x']+bounds['width']+12),40)
    await expect(dialog).to_have_count(0)
    await expect(trigger).to_be_focused()
    await trigger.click()
    await close.click()
    await expect(dialog).to_have_count(0)
    await expect(trigger).to_be_focused()
    await trigger.click()
    await dialog.get_by_role('link',name='Home',exact=True).click()
    await expect(dialog).to_have_count(0)
    # Selecting the current route must dismiss too.
    await trigger.click()
    await dialog.get_by_role('link',name='Home',exact=True).click()
    await expect(dialog).to_have_count(0)
    # Browser history changes also close a drawer without a link click inside it.
    await trigger.click()
    await dialog.get_by_role('link',name='About',exact=True).click()
    await page.wait_for_url('**/about')
    await expect(dialog).to_have_count(0)
    await trigger.click()
    await expect(dialog).to_be_visible()
    await page.go_back(wait_until='networkidle')
    await expect(dialog).to_have_count(0)
    assert urlsplit(page.url).path==('/home' if signed_in else '/')
    if prefix=='mobile':
        await trigger.click()
        await dialog.get_by_role('button',name='Toggle navigation theme',exact=True).click()
        await page.wait_for_function("document.documentElement.classList.contains('dark')")
        await page.screenshot(path=str(output/'mobile-drawer-dark.png'),full_page=False)
        await dialog.get_by_role('button',name='Toggle navigation theme',exact=True).click()
        await page.wait_for_function("!document.documentElement.classList.contains('dark')")
        await close.click()
        await page.set_viewport_size({'width':844,'height':390})
        await trigger.click()
        await dialog.get_by_role('button',name='Toggle navigation theme',exact=True).scroll_into_view_if_needed()
        assert await dialog.locator('.drawer-content').evaluate('e=>e.scrollTop>0')
        await page.screenshot(path=str(output/'mobile-drawer-landscape.png'),full_page=False)
        await close.click()
        await page.set_viewport_size({'width':390,'height':844})
    assert await page.evaluate("[document.documentElement,document.body].map(e=>[e.style.getPropertyValue('overflow'),e.style.getPropertyPriority('overflow')])")==original

async def check(args):
    from playwright.async_api import async_playwright,expect
    state,base,cookie_name=instance_metadata(args)
    url=urlsplit(base)
    if state!='ready' or url.scheme!='http' or url.hostname not in ('127.0.0.1','localhost','::1') or url.username or url.password or url.query or url.fragment or url.path not in ('','/'):
        raise ValueError('Browser smoke permits only a ready canonical loopback development instance')
    output=args.output.resolve()
    if output==ROOT or ROOT in output.parents:raise ValueError('Choose an artifact directory outside the source checkout')
    output.mkdir(parents=True,exist_ok=True,mode=0o700)
    label='SYNTHETIC template smoke '+str(time.time_ns())+' 100%_ & ideas'
    result={'passed':False,'checks':[],'page_errors':0,'synthetic_note_removed':False}
    phase='launch';note_url=None
    def passed(name,**metadata):
        result['checks'].append({'check':name,'passed':True,**metadata})
        print(json.dumps(result['checks'][-1]),flush=True)
    async with async_playwright() as p:
        executable=args.chromium or shutil.which('chromium') or shutil.which('chromium-browser')
        if not executable:raise ValueError('Install Chromium or provide --chromium PATH')
        options={'executable_path':executable,'headless':True,'args':['--disable-dev-shm-usage']}
        proxy=os.environ.get('HTTPS_PROXY') or os.environ.get('HTTP_PROXY')
        if proxy:options['proxy']={'server':proxy,'bypass':'127.0.0.1,localhost,[::1]'}
        browser=await p.chromium.launch(**options)
        context=await browser.new_context(viewport={'width':1440,'height':1000})
        page=await context.new_page();page.set_default_timeout(20000)
        page.on('pageerror',lambda error:result.update(page_errors=result['page_errors']+1))
        try:
            phase='home and theme hydration'
            response=await page.goto(base,wait_until='networkidle',timeout=60000)
            assert response.status==200
            await expect(page.get_by_role('heading',name='Make room for your next idea.',exact=True)).to_be_visible()
            await page.get_by_role('button',name='Toggle theme',exact=True).click()
            await page.wait_for_function("document.documentElement.classList.contains('dark')")
            await page.get_by_role('button',name='Toggle theme',exact=True).click()
            await page.wait_for_function("!document.documentElement.classList.contains('dark')")
            await page.screenshot(path=str(output/'desktop-landing.png'),full_page=True)
            passed('Public home and interactive theme hydration')
            phase='guest drawer keyboard, dismissal and navigation'
            await exercise_drawer(page,output,'desktop-guest',False)
            passed('Guest drawer focus trap, inert background, dismissal, restoration and same-route navigation')
            phase='public About and private route gates'
            await page.get_by_role('link',name='About',exact=True).click()
            await expect(page.get_by_role('heading',name='A little space for what matters.',exact=True)).to_be_visible()
            await page.screenshot(path=str(output/'desktop-about.png'),full_page=True)
            for target in ['/home','/dashboard','/search?q=private','/account','/settings','/notes/new']:
                await page.goto(base+target,wait_until='networkidle')
                await expect(page.get_by_role('heading',name='Welcome back',exact=True)).to_be_visible()
                assert urlsplit(page.url).path=='/login'
            passed('Public About and guest gates for every starter private page')
            phase='synthetic Google OAuth'
            await page.get_by_role('link',name='Continue with Google',exact=True).click()
            await page.locator('input[name="username"]').fill('template-browser-tester')
            await page.locator('input[type="submit"]').click()
            await page.wait_for_url(base+'/notes/new',timeout=30000)
            await page.wait_for_load_state('networkidle')
            await expect(page.get_by_role('heading',name='New note',exact=True)).to_be_visible()
            await page.get_by_role('link',name='Notes',exact=True).click()
            await expect(page.get_by_role('heading',name='My notes',exact=True)).to_be_visible()
            cookies=[cookie for cookie in await context.cookies() if cookie['name']==cookie_name]
            assert len(cookies)==1
            cookie=cookies[0]
            assert cookie['httpOnly'] and cookie['sameSite']=='Lax' and cookie['path']=='/' and not cookie['secure']
            assert cookie['domain']==url.hostname and 850<=cookie['expires']-time.time()<=901
            passed('Synthetic browser-bound OAuth and local cookie metadata')
            phase='authenticated drawer navigation'
            await exercise_drawer(page,output,'desktop',True)
            passed('Signed-in drawer destinations and navigation')
            phase='starter home, dashboard and empty search'
            await page.get_by_role('link',name='Home',exact=True).click()
            await expect(page.get_by_role('heading',name='Home',exact=True)).to_be_visible()
            await page.get_by_role('link',name='Dashboard',exact=True).click()
            await expect(page.get_by_role('heading',name='Dashboard',exact=True)).to_be_visible()
            await expect(page.get_by_test_id('total-notes')).to_be_visible()
            baseline_count=int(await page.get_by_test_id('total-notes').inner_text())
            await page.screenshot(path=str(output/'desktop-dashboard-initial.png'),full_page=True)
            await page.get_by_role('link',name='Search',exact=True).click()
            await expect(page.get_by_role('heading',name='What would you like to find?',exact=True)).to_be_visible()
            passed('Distinct signed-in Home, Dashboard statistics and initial Search state')
            phase='create note'
            await page.get_by_role('link',name='New note',exact=True).click()
            phase='fill note title'
            await page.get_by_label('Title',exact=True).fill(label)
            phase='fill note body'
            await page.get_by_label('Note',exact=True).fill('Synthetic local test data; removed by this smoke test.')
            phase='submit note'
            await page.get_by_role('button',name='Save note',exact=True).click()
            phase='wait for saved note route'
            await page.wait_for_url(re.compile(re.escape(base)+r'/notes/[0-9a-f-]+$'),timeout=30000)
            note_url=page.url
            await page.reload(wait_until='networkidle')
            await expect(page.get_by_label('Title',exact=True)).to_have_value(label)
            passed('Create/save/reopen private note')
            phase='edit note'
            await page.get_by_label('Note',exact=True).fill('Edited synthetic local test data.')
            await page.get_by_role('button',name='Save note',exact=True).click()
            await expect(page.locator('main form').get_by_role('status')).to_contain_text('Saved')
            response=await page.reload(wait_until='networkidle')
            await expect(page.get_by_label('Note',exact=True)).to_have_value('Edited synthetic local test data.')
            assert response.headers.get('cache-control')=='private, no-store'
            await page.screenshot(path=str(output/'desktop-note.png'),full_page=True)
            passed('Edit/reopen note and private response caching')
            phase='dashboard and home data freshness'
            await page.get_by_role('link',name='Dashboard',exact=True).click()
            await expect(page.get_by_test_id('total-notes')).to_have_text(str(baseline_count+1))
            await expect(page.get_by_role('heading',name=label,exact=True)).to_be_visible()
            await page.get_by_role('button',name='Refresh',exact=True).click()
            await expect(page.get_by_test_id('total-notes')).to_have_text(str(baseline_count+1))
            await page.screenshot(path=str(output/'desktop-dashboard.png'),full_page=True)
            await page.get_by_role('link',name='Home',exact=True).click()
            await expect(page.get_by_role('heading',name=label,exact=True)).to_be_visible()
            await page.screenshot(path=str(output/'desktop-home.png'),full_page=True)
            passed('Home recent notes and live Dashboard totals reflect the saved note')
            phase='literal search, reload, history and open result'
            await page.get_by_role('link',name='Search',exact=True).click()
            await page.get_by_label('Search notes',exact=True).fill(label)
            await page.get_by_role('button',name='Search notes',exact=True).click()
            await expect(page.get_by_role('heading',name=label,exact=True)).to_be_visible()
            assert parse_qs(urlsplit(page.url).query).get('q')==[label]
            await page.reload(wait_until='networkidle')
            await expect(page.get_by_label('Search notes',exact=True)).to_have_value(label)
            await expect(page.get_by_role('heading',name=label,exact=True)).to_be_visible()
            await page.screenshot(path=str(output/'desktop-search.png'),full_page=True)
            await page.get_by_role('link',name='Clear search',exact=True).click()
            await expect(page.get_by_label('Search notes',exact=True)).to_have_value('')
            await expect(page.get_by_role('heading',name='What would you like to find?',exact=True)).to_be_visible()
            await page.go_back(wait_until='networkidle')
            await expect(page.get_by_label('Search notes',exact=True)).to_have_value(label)
            await expect(page.get_by_role('heading',name=label,exact=True)).to_be_visible()
            await page.get_by_role('heading',name=label,exact=True).click()
            await page.wait_for_url(note_url)
            await expect(page.get_by_label('Note',exact=True)).to_have_value('Edited synthetic local test data.')
            passed('Literal special-character search, URL/reload/back history and result navigation')
            phase='body-only search and no matches'
            await page.goto(base+'/search?q='+quote('Edited synthetic local test data.',safe=''),wait_until='networkidle')
            await expect(page.get_by_role('heading',name=label,exact=True)).to_be_visible()
            await page.goto(base+'/search?q='+quote('no-match-'+label,safe=''),wait_until='networkidle')
            await expect(page.get_by_role('heading',name='No notes found',exact=True)).to_be_visible()
            passed('Body search and empty result state')
            phase='another account cannot search or open the note'
            other=await browser.new_context(viewport={'width':1280,'height':900})
            op=await other.new_page()
            op.on('pageerror',lambda error:result.update(page_errors=result['page_errors']+1))
            await op.goto(base+'/search?q='+quote(label,safe=''),wait_until='networkidle')
            await op.get_by_role('link',name='Continue with Google',exact=True).click()
            await op.locator('input[name="username"]').fill('template-other-'+str(time.time_ns()))
            await op.locator('input[type="submit"]').click()
            await expect(op.get_by_role('heading',name='No notes found',exact=True)).to_be_visible()
            await op.goto(note_url,wait_until='networkidle')
            await expect(op.get_by_role('heading',name='Note unavailable',exact=True)).to_be_visible()
            await op.goto(base+'/dashboard',wait_until='networkidle')
            await expect(op.get_by_test_id('total-notes')).to_have_text('0')
            await op.get_by_role('button',name='Sign out',exact=True).click()
            await op.wait_for_load_state('networkidle')
            await other.close()
            passed('Separate synthetic account has isolated Search, Dashboard and note access')
            phase='mobile layout and dialog interaction'
            mobile=await browser.new_context(storage_state=await context.storage_state(),viewport={'width':390,'height':844},is_mobile=True,has_touch=True)
            mp=await mobile.new_page()
            mp.on('pageerror',lambda error:result.update(page_errors=result['page_errors']+1))
            await mp.goto(note_url,wait_until='networkidle')
            await expect(mp.get_by_label('Title',exact=True)).to_have_value(label)
            await mp.screenshot(path=str(output/'mobile-note.png'),full_page=True)
            overflow=await mp.evaluate('document.documentElement.scrollWidth>innerWidth')
            if overflow:
                print(json.dumps({'mobile_overflow':await mp.evaluate("Array.from(document.querySelectorAll('body *')).filter(e=>e.getBoundingClientRect().right>innerWidth+1).map(e=>({tag:e.tagName,cls:e.className,width:Math.round(e.getBoundingClientRect().width)})).slice(0,12)")}),flush=True)
            assert not overflow
            await mp.get_by_role('button',name='Delete note',exact=True).click()
            await expect(mp.get_by_role('dialog')).to_be_visible()
            await mp.screenshot(path=str(output/'mobile-dialog.png'),full_page=True)
            await mp.get_by_role('button',name='Keep note',exact=True).click()
            await expect(mp.get_by_role('dialog')).to_have_count(0)
            for path,heading,filename in [
                ('/home','Home','mobile-home.png'),
                ('/dashboard','Dashboard','mobile-dashboard.png'),
                ('/search?q='+quote(label,safe=''),'Search','mobile-search.png'),
                ('/account','Your account','mobile-account.png'),
                ('/settings','Settings','mobile-settings.png'),
                ('/about','A little space for what matters.','mobile-about.png'),
            ]:
                await mp.goto(base+path,wait_until='networkidle')
                await expect(mp.get_by_role('heading',name=heading,exact=True)).to_be_visible()
                if path=='/dashboard':await expect(mp.get_by_test_id('total-notes')).to_have_text(str(baseline_count+1))
                if path=='/home' or path.startswith('/search?'):await expect(mp.get_by_role('heading',name=label,exact=True)).to_be_visible()
                assert not await mp.evaluate('document.documentElement.scrollWidth>innerWidth')
                await mp.screenshot(path=str(output/filename),full_page=True)
            phase='mobile drawer and landscape scrolling'
            await exercise_drawer(mp,output,'mobile',True)
            passed('Mobile drawer, shared theme and short-viewport scrolling')
            await mobile.close()
            passed('Mobile editor/dialog and all six starter pages without horizontal overflow')
            phase='settings appearance persistence and account overview'
            await page.get_by_role('link',name='Settings',exact=True).click()
            await expect(page.get_by_role('heading',name='Settings',exact=True)).to_be_visible()
            await page.get_by_role('button',name='Dark appearance',exact=True).click()
            await page.wait_for_function("document.documentElement.classList.contains('dark')")
            await expect(page.get_by_role('button',name='Toggle theme',exact=True)).to_have_text('Light')
            await page.reload(wait_until='networkidle')
            await expect(page.get_by_role('button',name='Dark appearance',exact=True)).to_have_attribute('aria-pressed','true')
            await page.screenshot(path=str(output/'desktop-settings-dark.png'),full_page=True)
            await page.get_by_role('button',name='Light appearance',exact=True).click()
            await page.wait_for_function("!document.documentElement.classList.contains('dark')")
            await expect(page.get_by_role('button',name='Toggle theme',exact=True)).to_have_text('Dark')
            await page.get_by_role('link',name='Account',exact=True).click()
            await expect(page.get_by_role('heading',name='Your account',exact=True)).to_be_visible()
            await page.screenshot(path=str(output/'desktop-account.png'),full_page=True)
            passed('Settings shares persisted theme state with the header and links to Account')
            phase='profile persistence'
            await page.get_by_role('link',name='Settings',exact=True).click()
            await page.get_by_role('link',name='Profile',exact=True).click()
            old_name=(await page.get_by_label('Display name',exact=True).input_value()).strip() or 'Local template tester'
            await page.get_by_label('Display name',exact=True).fill('Template browser tester')
            await page.get_by_role('button',name='Save profile',exact=True).click()
            await expect(page.locator('main form').get_by_role('status')).to_contain_text('Profile saved')
            await page.reload(wait_until='networkidle')
            await expect(page.get_by_label('Display name',exact=True)).to_have_value('Template browser tester')
            await page.get_by_role('link',name='Account',exact=True).click()
            await expect(page.get_by_role('heading',name='Template browser tester',exact=True)).to_be_visible()
            await page.get_by_role('link',name='Profile',exact=True).click()
            await page.get_by_label('Display name',exact=True).fill(old_name)
            await page.get_by_role('button',name='Save profile',exact=True).click()
            await expect(page.locator('main form').get_by_role('status')).to_contain_text('Profile saved')
            passed('Profile save/reload and restoration')
            phase='last sign-in method guard'
            await page.get_by_role('link',name='Sign-in methods',exact=True).click()
            await expect(page.get_by_role('heading',name='Sign-in methods',exact=True)).to_be_visible()
            controls=page.get_by_role('button',name='Unlink',exact=True)
            if await controls.count()==1:await expect(controls).to_be_disabled()
            passed('Accounts page and last-method UI guard')
            phase='link and unlink synthetic GitHub identity'
            await page.get_by_role('link',name='Link GitHub',exact=True).click()
            await page.locator('input[name="username"]').fill('template-github-tester')
            await page.locator('input[type="submit"]').click()
            await page.wait_for_url(base+'/settings/accounts',timeout=30000)
            await page.wait_for_load_state('networkidle')
            await expect(page.get_by_role('button',name='Unlink',exact=True)).to_have_count(2)
            github_row=page.locator('.account-list li').filter(has=page.get_by_text('github',exact=True))
            await github_row.get_by_role('button',name='Unlink',exact=True).click()
            await expect(page.get_by_role('button',name='Unlink',exact=True)).to_have_count(1)
            await expect(page.get_by_role('button',name='Unlink',exact=True)).to_be_disabled()
            passed('Explicit synthetic GitHub linking/unlinking preserves Google sign-in')
            phase='delete synthetic note'
            await page.goto(note_url,wait_until='networkidle')
            await page.get_by_role('button',name='Delete note',exact=True).click()
            await page.get_by_role('button',name='Delete permanently',exact=True).click()
            await page.wait_for_url(base+'/notes',timeout=30000)
            await page.reload(wait_until='networkidle')
            assert await page.get_by_text(label,exact=True).count()==0
            result['synthetic_note_removed']=True
            passed('Synthetic note deletion and list verification')
            await page.get_by_role('link',name='Dashboard',exact=True).click()
            await expect(page.get_by_test_id('total-notes')).to_have_text(str(baseline_count))
            await page.goto(base+'/search?q='+quote(label,safe=''),wait_until='networkidle')
            await expect(page.get_by_role('heading',name='No notes found',exact=True)).to_be_visible()
            passed('Dashboard and Search reflect deletion without stale results')
            phase='logout and server-side session revocation'
            old_state=await context.storage_state() # In-memory only, never serialized to disk/logs.
            await page.get_by_role('button',name='Open navigation menu',exact=True).click()
            async with page.expect_response(lambda response:urlsplit(response.url).path=='/auth/logout') as logout:
                await page.get_by_role('dialog',name='Navigation',exact=True).get_by_role('button',name='Sign out',exact=True).click()
            assert (await logout.value).status in (302,303)
            await page.wait_for_load_state('networkidle')
            assert not any(cookie['name']==cookie_name for cookie in await context.cookies())
            await page.goto(base+'/notes/new',wait_until='networkidle')
            await expect(page.get_by_role('heading',name='Welcome back',exact=True)).to_be_visible()
            replay=await browser.new_context(storage_state=old_state)
            rp=await replay.new_page();await rp.goto(base+'/notes/new',wait_until='networkidle')
            await expect(rp.get_by_role('heading',name='Welcome back',exact=True)).to_be_visible()
            await replay.close()
            await page.screenshot(path=str(output/'desktop-signed-out.png'),full_page=True)
            passed('Native logout expires cookie and revoked-cookie replay cannot create notes')
            assert result['page_errors']==0
            result['passed']=True
        except Exception as error:
            result.update(failed_phase=phase,error_type=type(error).__name__)
            await page.screenshot(path=str(output/'failure.png'),full_page=True)
            print(json.dumps({'failed_phase':phase,'error_type':type(error).__name__,'path':urlsplit(page.url).path,'headings':await page.locator('h1').all_text_contents(),'feedback':await page.get_by_role('status').all_text_contents()}),flush=True)
        finally:
            if note_url and not result['synthetic_note_removed']:
                try:
                    await page.goto(note_url,wait_until='networkidle')
                    await page.get_by_role('button',name='Delete note',exact=True).click(timeout=3000)
                    await page.get_by_role('button',name='Delete permanently',exact=True).click(timeout=3000)
                    await page.wait_for_url(base+'/notes',timeout=10000)
                    result['synthetic_note_removed']=True
                except Exception:result['cleanup_pending_note_path']=urlsplit(note_url).path
            await browser.close()
            (output/'result.json').write_text(json.dumps(result,indent=2)+'\n')
    if not result['passed']:raise ValueError('Browser smoke failed; inspect the external result and screenshots')
    print('Browser smoke passed. Artifacts:',output)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--instance',required=True);parser.add_argument('--host',action='store_true')
    parser.add_argument('--chromium');parser.add_argument('--output',type=Path,default=Path(os.environ.get('APP_BROWSER_OUTPUT',str(Path(tempfile.gettempdir())/'app-browser-artifacts'))))
    args=parser.parse_args()
    try:asyncio.run(check(args))
    except Exception as error:parser.exit(1,'Browser validation failed: '+type(error).__name__+'\n')
if __name__=='__main__':main()
