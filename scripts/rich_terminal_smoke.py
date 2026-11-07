"""Native ConPTY selection/copy acceptance with a private Windows clipboard station.

The Rust ignored-test entry runs the real desk and native clipboard worker in an
isolated window station. It reads back ONLY that station's synthetic clipboard;
the interactive desktop clipboard and personal configuration are never touched.
"""
import argparse,hashlib,json,os,re,tempfile,time
from pathlib import Path
from desk_terminal_smoke import Terminal
from qq_memory_smoke import Smoke,require

RAW="# 阅读与复制\n\nselect_this_text 是 **重点**，还有 `inline`。\n\n" + "中文段落会自动折行，复制时应保持原句。"*8 + "\n\n> 引用仍可阅读。\n\n```python\n\tprint('│ and | stay')\n    x = 1\n```\n\n```rust\nlet value = 42;\nprintln!(\"{}\", value);\n```"
CODE1="\tprint('│ and | stay')\n    x = 1\n"
CODE2='let value = 42;\nprintln!("{}", value);\n'

def main():
    global RAW
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir',type=Path,required=True)
    parser.add_argument('--test-exe',type=Path,required=True)
    parser.add_argument('--artifacts',type=Path,required=True)
    parser.add_argument('--portrait-file',type=Path)
    parser.add_argument('--long-history',action='store_true')
    args=parser.parse_args()
    if args.long_history:
        RAW += '\n\n' + ''.join(f'MARKER {i:02} drag_token_{i:02}\n\nParagraph {i:02} contains Chinese 中文 and **bold words**.\n\n' for i in range(30))
    require(os.name=='nt','native Windows only; other platforms must be checked natively')
    artifacts=args.artifacts.resolve();require(not artifacts.is_relative_to(Path(__file__).resolve().parents[1]),'artifacts outside checkout')
    artifacts.mkdir(parents=True,exist_ok=True)
    smoke=terminal=None
    with tempfile.TemporaryDirectory(prefix='lain-ui-terminal-') as directory:
        try:
            root=Path(directory);smoke=Smoke(root,args.bin_dir.resolve(),None,300);smoke.setup(None)
            smoke.env.pop('LAIN_NAPCAT_CONFIG',None);smoke.env.pop('LAIN_PORTRAIT_FILE',None)
            if args.portrait_file:smoke.env['LAIN_PORTRAIT_FILE']=str(args.portrait_file.resolve())
            smoke.fixture.respond=lambda _: [{'type':'message','role':'assistant','content':[{'type':'output_text','text':RAW}]}]
            smoke.start();conversation=smoke.call('POST','/v1/conversations',expected=201)['id']
            receipt=root/'clipboard-receipt.jsonl'
            env=smoke.env|{'LAIN_TERMINAL_COPY_FIXTURE':'isolated-fixture','LAIN_TERMINAL_COPY_RECEIPT':str(receipt),'LAIN_TERMINAL_COPY_CONVERSATION':conversation}
            terminal=Terminal([str(args.test_exe.resolve()),'--exact','desk::clipboard::tests::native_terminal_private_clipboard','--ignored','--nocapture'],env)
            def visible(text):smoke.wait(lambda:text in terminal.text(),'missing terminal text '+text)
            def copies():return [json.loads(line) for line in receipt.read_text(encoding='utf-8').splitlines()] if receipt.exists() else []
            def copied(count):smoke.wait(lambda:len(copies())==count,'native clipboard copy not confirmed',seconds=15)
            def capture(name):time.sleep(.12);terminal.image(artifacts/(name+'.png'))
            def locate(text):
                from wcwidth import wcswidth
                for y,row in enumerate(terminal.text().splitlines()):
                    if text in row:return wcswidth(row[:row.index(text)]),y
                raise AssertionError('visible glyph not found '+text)
            visible('lain // wired')
            terminal.write('rich fixture\r')
            smoke.wait(lambda:any(t['state']=='completed' for t in smoke.call('GET',f'/v1/conversations/{conversation}?recent=true')['turns']),'fixture turn not complete')
            if args.long_history:
                terminal.write('\x1b[1;5H');visible('select_this_text')
            else:visible('println!')
            terminal.write('draft retained')
            terminal.write('\x0f');copied(1)
            require('**' not in copies()[0] and 'select_this_text' in copies()[0] and 'Lain>' not in copies()[0],'plain copy contained presentation syntax')
            capture('rich-code-and-copy')
            for option,expected in [(1,RAW),(2,CODE1),(3,CODE2)]:
                terminal.write('\x1bOQ');visible('Copy ·')
                if option==1:capture('copy-menu')
                terminal.write('\x1b[B'*option+'\r');copied(option+1)
                require(copies()[-1]==expected,'native clipboard changed exact Markdown/code bytes')
            with terminal.lock:before_frame=len(terminal.output)
            terminal.write('\x1b[1;5H') # Ctrl-Home.
            # The marker may already be visible before scrolling. Wait for the
            # requested frame before deriving physical mouse coordinates.
            def next_frame():
                with terminal.lock:return len(terminal.output)>before_frame
            smoke.wait(next_frame,'Ctrl-Home did not render',seconds=10)
            time.sleep(.15)
            visible('select_this_text')
            require('**重点**' not in terminal.text(),'Markdown emphasis was rendered literally')
            x,y=locate('select_this_text')
            terminal.write(f'\x1b[<0;{x+1};{y+1}M\x1b[<32;{x+len("select_this_text")};{y+1}M\x1b[<0;{x+len("select_this_text")};{y+1}m')
            terminal.write('\x03');copied(5)
            require(copies()[-1]=='select_this_text','drag selection copied borders, role or sidebar text')
            visible('draft retained');capture('selected-text')
            terminal.write('\x1bOR');visible('只读阅读') # F3.
            terminal.write('must not enter draft')
            capture('clean-reader')
            terminal.write('\x1b');visible('draft retained')
            require('must not enter draft' not in terminal.text(),'reader altered chat input')
            if args.long_history:
                terminal.write('\x1b[1;5F');visible('MARKER 29')
                seen=set();drag_checked=False
                for _ in range(30):
                    screen=terminal.text()
                    seen.update(re.findall(r'MARKER (\d+)',screen))
                    caption=next(y for y,row in enumerate(screen.splitlines()) if 'latest' in row and ('Reading history' in row or 'Ctrl-End' in row)) if 'Reading history' in screen else next(y for y,row in enumerate(screen.splitlines()) if 'Conversation / result' in row)
                    top=caption+1
                    if not drag_checked and not screen.splitlines()[top].strip():
                        token=re.search(r'drag_token_\d+',screen)
                        if token:
                            value=token.group();x,y=locate(value)
                            terminal.write(f'\x1b[<0;{x+1};{y+1}M')
                            time.sleep(.08) # Let the real frame redraw between press and movement.
                            terminal.write(f'\x1b[<32;{x+len(value)};{y+1}M\x1b[<0;{x+len(value)};{y+1}m\x03')
                            copied(6)
                            require(copies()[-1]==value,'drag on a scrolled page jumped or copied a distant paragraph')
                            drag_checked=True;capture('long-history-stable-selection')
                    x,y=locate('draft retained')
                    # Use a point inside the body rather than the composer/portrait.
                    terminal.write(f'\x1b[<64;80;{top+3}M')
                    time.sleep(.06)
                require(len(seen)>=20,'wheel got stuck while traversing blank-separated paragraphs')
                require(drag_checked,'fixture did not exercise a blank line at the viewport top')
                capture('long-history-wheel-traversal')
            require(len(smoke.fixture.requests)==1,'copy/read actions called the model')
            terminal.write('\x11');smoke.wait(lambda:not terminal.process.isalive(),'fixture desk failed to exit')
            stream=''.join(terminal.output)
            require(not any(secret in stream for secret in smoke.credentials),'credential in terminal stream')
            values=copies();(artifacts/'synthetic-clipboard-receipts.json').write_text(json.dumps(values,ensure_ascii=False,indent=2),encoding='utf-8')
            report={'platform':'native Windows ConPTY + private clipboard window station','paid_calls':0,'fake_model_calls':1,
                'checks':{'plain_response':True,'exact_markdown':True,'exact_code_indentation_and_symbols':True,'second_code_block':True,'native_drag_selected_text':True,'reader_draft_retained':True,'no_model_or_worker_effect_for_copy':True,'long_history_wheel_and_drag':args.long_history},
                'test_executable_sha256':hashlib.sha256(args.test_exe.read_bytes()).hexdigest(),'client_sha256':hashlib.sha256((args.bin_dir/'lain.exe').read_bytes()).hexdigest(),'personal_clipboard_touched':False,
                'images':'actual ConPTY VT stream rendered with fonts; synthetic message/work content'}
            smoke.fixture.healthy()
        except Exception:
            if terminal:
                (artifacts/'failure.txt').write_text(terminal.text(),encoding='utf-8');terminal.image(artifacts/'failure.png')
                (artifacts/'failure-vt.txt').write_text(''.join(terminal.output),encoding='utf-8')
            raise
        finally:
            if terminal:terminal.close()
            if smoke:
                try:smoke.stop()
                finally:
                    if smoke.fixture:smoke.fixture.close()
    report['temporary_state_cleaned']=not root.exists()
    require(report['temporary_state_cleaned'],'fixture directory was retained')
    (artifacts/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8');print(json.dumps(report,indent=2),flush=True)

if __name__=='__main__':main()
