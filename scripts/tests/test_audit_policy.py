import os,subprocess,tempfile,unittest
from pathlib import Path
SCRIPT=Path(__file__).resolve().parents[1]/'security-audit.sh'
class AuditPolicyTests(unittest.TestCase):
 def run_policy(self,entries):
  with tempfile.TemporaryDirectory() as directory:
   path=Path(directory)/'allowlist.txt';path.write_text(entries)
   env=os.environ.copy();env['APP_AUDIT_ALLOWLIST_FILE']=str(path)
   return subprocess.run(['bash',str(SCRIPT),'--validate-only','--today','2026-10-31'],env=env,capture_output=True,text=True).returncode
 def test_narrow_explained_unexpired_entry_is_accepted(self):
  self.assertEqual(self.run_policy('RUSTSEC-2023-0071|2026-11-30|Not in production graph\n'),0)
 def test_expired_or_unexplained_exception_is_rejected(self):
  for entry in ['RUSTSEC-2023-0071|2026-10-01|Old reason','RUSTSEC-2023-0071|2026-11-30|']:
   with self.subTest(entry=entry):self.assertNotEqual(self.run_policy(entry),0)
 def test_duplicate_or_invalid_advisory_is_rejected(self):
  entry='RUSTSEC-2023-0071|2026-11-30|Reason\n'
  self.assertNotEqual(self.run_policy(entry+entry),0)
  self.assertNotEqual(self.run_policy('*|2026-11-30|Too broad\n'),0)
if __name__=='__main__':unittest.main()
