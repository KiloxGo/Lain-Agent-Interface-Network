"""Owner-only files for explicit local acceptance artifacts; no runtime access."""
import os
from pathlib import Path
import subprocess


def protect_directory(path):
    path = Path(path)
    if not path.is_dir() or path.is_symlink() or (os.name == "nt" and path.is_junction()):
        raise ValueError("acceptance directory must be a real owned directory")
    if os.name != "nt":
        path.chmod(0o700)
        return
    script = r"""
$ErrorActionPreference='Stop'
$env:PSModulePath=[IO.Path]::Combine($PSHOME,'Modules')
$p=$env:LAIN_ACCEPTANCE_PATH
$sid=[Security.Principal.WindowsIdentity]::GetCurrent().User
$a=New-Object Security.AccessControl.DirectorySecurity
$a.SetOwner($sid)
$a.SetAccessRuleProtection($true,$false)
$a.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule($sid,'FullControl','ContainerInherit,ObjectInherit','None','Allow')))
[IO.Directory]::SetAccessControl($p,$a)
$b=Get-Acl -LiteralPath $p
if (-not $b.AreAccessRulesProtected -or $b.GetOwner([Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) {throw 'owner ACL'}
foreach($r in $b.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])) {if($r.IdentityReference.Value -ne $sid.Value -or $r.AccessControlType -ne 'Allow'){throw 'non-owner ACL'}}
"""
    env = {k: v for k, v in os.environ.items() if k.upper() in {
        "SYSTEMROOT", "WINDIR", "PATH", "TEMP", "TMP", "USERPROFILE", "LOCALAPPDATA", "APPDATA", "COMSPEC", "PATHEXT"}}
    env["LAIN_ACCEPTANCE_PATH"] = str(path)
    result = subprocess.run(["powershell.exe", "-NoProfile", "-NonInteractive", "-Command", script],
                            env=env, capture_output=True, timeout=30)
    if result.returncode:
        raise RuntimeError("owner-only acceptance ACL failed: " + result.stderr.decode(errors="replace")[-1200:])
