#!/usr/bin/env python3
"""Native isolated daily-evidence CLI/API check. No live sources or model calls."""
import argparse
import json
import os
from pathlib import Path
import tempfile
import time
import uuid

from qq_memory_smoke import Smoke, require


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir',type=Path,required=True)
    parser.add_argument('--artifacts',type=Path,required=True)
    args=parser.parse_args()
    artifacts=args.artifacts.resolve()
    require(not artifacts.is_relative_to(Path(__file__).resolve().parents[1]),'artifacts outside checkout')
    artifacts.mkdir(parents=True,exist_ok=True)
    smoke=None
    with tempfile.TemporaryDirectory(prefix='lain-qq-memory-briefing-') as directory:
        try:
            smoke=Smoke(Path(directory),args.bin_dir.resolve(),None,600)
            smoke.setup(None)
            smoke.env.pop('LAIN_NAPCAT_CONFIG',None)
            smoke.start()
            conversation=smoke.call('POST','/v1/conversations',expected=201)['id']
            other=smoke.call('POST','/v1/conversations',expected=201)['id']
            source=smoke.call('POST','/v1/archives/sources',{'kind':'qq_group','observer_account':'1001','group_id':'3003'})['source']['id']
            now=int(time.time()*1000)
            today=((now+480*60000)//86400000)*86400000-480*60000
            yesterday=today-86400000
            smoke.call('POST',f'/v1/archives/sources/{source}/grant',{'expected_revision':0,'scope':{'expires_at_ms':now+600000,'retention_ms':86400000,'max_records':100}})
            rows=[]
            for offset in (0,32):
                inputs=[{'external_id':f'synthetic-{i}','source_position':None,'occurred_at_ms':yesterday+i*1000,'speaker':'2002','kind':'message','text':f'SYNTHETIC-DAY-{i}：明晚开会；这只是待解读的原文。'} for i in range(offset,min(offset+32,43))]
                rows.extend(smoke.call('POST',f'/v1/archives/sources/{source}/import',{'expected_revision':1,'records':inputs})['records'])
            grant={'request_id':str(uuid.uuid4()),'expected_revision':0,'scope':{'sources':[{'source_id':source,'grant_revision':1}],'from_ms':0,'expires_at_ms':now+300000,'utc_offset_minutes':480,'max_records':100}}
            path=Path(directory)/'reviewed-grant.json'
            path.write_text(json.dumps(grant),encoding='utf-8')
            require(smoke.cli('archive','briefing-grant',conversation)['scope'] is None,'grant default off')
            require(smoke.cli('archive','briefing-enable',conversation,'--file',path)['revision']==1,'explicit grant')
            request=str(uuid.uuid4())
            manifest=smoke.cli('archive','briefing-prepare',conversation,'--request-id',request,'--revision','1')
            require(len(manifest['records'])==43 and manifest['day_from_ms']==yesterday and manifest['day_to_ms']==today,'whole selected day')
            require('SYNTHETIC-DAY-' not in json.dumps(manifest),'manifest stores no source body')
            require(not manifest['complete_upstream_history'],'no upstream completeness claim')
            selected=[]
            index=0
            pages=0
            while True:
                page=smoke.cli('archive','briefing-page',conversation,request,'--from-index',str(index))
                require(len(page['records'])<=8 and page['text_bytes']<=16384,'page bound')
                selected.extend(r['id'] for r in page['records'])
                pages+=1
                if page['next_index'] is None:break
                require(page['next_index']>index,'cursor advances')
                index=page['next_index']
            require(selected==[r['id'] for r in rows] and pages==6,'all pages preserve identity order')
            smoke.call('GET',f'/v1/briefing/{other}/manifests/{request}/page',expected=404)
            smoke.call('GET',f'/v1/briefing/{conversation}/manifests/{request}/page',auth=False,expected=401)
            smoke.stop()
            smoke.start()
            again=smoke.cli('archive','briefing-prepare',conversation,'--request-id',request,'--revision','1')
            require(again==manifest,'restart exact receipt changes day or records')
            smoke.call('POST',f'/v1/archives/records/{rows[-1]["id"]}/correct',{'expected_revision':rows[-1]['revision'],'text':'SYNTHETIC correction: notice time changed.'})
            smoke.call('GET',f'/v1/briefing/{conversation}/manifests/{request}/page',expected=409)
            fresh=smoke.cli('archive','briefing-prepare',conversation,'--request-id',str(uuid.uuid4()),'--revision','1')
            smoke.cli('archive','briefing-revoke',conversation,'--request-id',str(uuid.uuid4()),'--revision','1')
            smoke.call('GET',f'/v1/briefing/{conversation}/manifests/{fresh["request"]["request_id"]}/page',expected=409)
            require(smoke.call('GET',f'/v1/archives/sources/{source}')['grant']['scope'] is not None,'briefing revoke must not silently revoke import')
            require(not smoke.fixture.requests,'evidence preparation called model')
            require(not smoke.call('GET',f'/v1/conversations/{conversation}?recent=true')['turns'],'fabricated Actor turn')
            report={'native_windows':os.name=='nt','selected_records':43,'pages':pages,'passed':['default_off','explicit_conversation_grant','full_day_six_pages','text_free_manifest','cross_conversation_denied','owner_auth','restart_exact_receipt','any_selected_correction_invalidates_first_page','independent_read_revoke','no_model_or_actor_effect'],'model_calls':0,'live_qq':False,'scope':'evidence_preparation_only_not_generated_briefing_or_memory'}
        finally:
            if smoke:
                smoke.stop()
                if smoke.fixture:smoke.fixture.close()
    report['temporary_state_cleaned']=True
    (artifacts/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    print(json.dumps(report),flush=True)


if __name__=='__main__':main()
