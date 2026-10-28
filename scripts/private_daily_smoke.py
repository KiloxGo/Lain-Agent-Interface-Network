"""Native private-daily admission smoke using fake OneBot/model only; no paid calls."""
import argparse,json,tempfile,hashlib,sqlite3
from contextlib import closing
from pathlib import Path
from qq_memory_smoke import Smoke,executable,require
from acceptance_files import protect_directory

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir',type=Path,required=True)
    args=parser.parse_args()
    temporary=tempfile.TemporaryDirectory(prefix='lain-private-daily-')
    root=Path(temporary.name).resolve();protect_directory(root)
    smoke=None
    try:
        smoke=Smoke(root,args.bin_dir.resolve(),None,180)
        smoke.setup(None);smoke.fixture.login_release.set()
        smoke.fixture.respond=lambda body:[{'type':'message','role':'assistant','content':[{'type':'output_text','text':'synthetic private reply'}]}]
        smoke.start()
        smoke.channel=smoke.wait(lambda:next(iter(smoke.call('GET','/v1/channels')),None),'binding missing')['id']
        import time,uuid
        old=str(uuid.uuid4())
        smoke.cli('inference','enable','--revision',0,'--id',old,'--max-requests',1,'--expires-at-ms',int(time.time()*1000)+60000)
        smoke.input_turn('finite fixture')
        require(smoke.cli('inference','show')['admission']=='exhausted','finite ceiling not spent')
        smoke.call('POST','/v1/inference/budget',{'expected_revision':1,'limit':None,'unlimited_private_channel':smoke.channel},auth=False,expected=401)
        smoke.cli('inference','private-daily',smoke.channel,'--revision',1)
        smoke.input_turn('daily fixture one')
        smoke.stop();smoke.start()
        state=smoke.cli('inference','show')
        require(state['admission']=='private_conversation_only' and state['remaining_requests'] is None,'daily policy did not persist')
        smoke.input_turn('daily fixture two')
        smoke.input_turn('daily fixture three')
        require(len(smoke.fixture.requests)==4,'normal private reply did not use exactly one model request')
        budgets=smoke.cli('inference','budgets')
        require(next(b for b in budgets if b['limit']['id']==old)['reserved_requests']==1,'daily policy refunded prior finite usage')
        c=smoke.call('POST','/v1/conversations',expected=201)
        t=smoke.call('POST',f"/v1/conversations/{c['id']}/turns",{'request_id':str(uuid.uuid4()),'text':'unbound fixture must be blocked'},expected=202)
        smoke.wait(lambda:(d if (d:=smoke.call('GET',f"/v1/actor-turns/{t['id']}"))['state']!='running' else None),'unbound denial did not settle')
        require(len(smoke.fixture.requests)==4,'private daily authorized an unbound conversation')
        smoke.cli('inference','stop','--revision',state['decision']['revision'])
        require(smoke.cli('inference','show')['admission']=='stopped','daily stop failed')
        smoke.fixture.healthy();smoke.stop()
        with closing(sqlite3.connect(smoke.local/'state.db')) as db:
            schema=db.execute('PRAGMA user_version').fetchone()[0]
        result={'checks':{'native_cli_auth_scope':True,'native_private_replies_after_finite_exhaustion':True,'native_restart_preserves_mode_and_usage':True,'native_other_conversation_denied_and_stop':True},'fake_model_requests':4,'paid_calls':0,'real_qq_sends':0,'schema':schema}
    finally:
        if smoke:
            smoke.stop()
            if smoke.fixture:
                smoke.fixture.model_release.set();smoke.fixture.login_release.set();smoke.fixture.close()
        require(root.is_relative_to(Path(tempfile.gettempdir()).resolve()) and root.name.startswith('lain-private-daily-'),'unsafe temporary cleanup')
        temporary.cleanup()
    require(not root.exists(),'temporary state survived')
    result['cleanup']=True
    result['binary_sha256']={n:hashlib.sha256((args.bin_dir/executable(n)).read_bytes()).hexdigest() for n in ['lain','laind','lain-test-app-server']}
    print(json.dumps(result))

if __name__=='__main__':main()
