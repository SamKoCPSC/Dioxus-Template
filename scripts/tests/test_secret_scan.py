import os,subprocess,tempfile,unittest
from pathlib import Path
SCANNER=Path(__file__).resolve().parents[1]/'check-tracked-secrets.sh'
class SecretScanTests(unittest.TestCase):
 def scan(self,root):
  env=os.environ.copy();env['APP_SECRET_SCAN_ROOT']=str(root)
  return subprocess.run(['bash',str(SCANNER)],env=env,text=True,capture_output=True)
 def test_examples_allow_only_documented_placeholders_and_never_echo_values(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);subprocess.run(['git','init','-q',str(root)],check=True)
   sample=root/'.env.production.example';sample.write_text('GOOGLE_CLIENT_SECRET=YOUR_GOOGLE_CLIENT_SECRET\n')
   subprocess.run(['git','-C',str(root),'add','.env.production.example'],check=True)
   self.assertEqual(self.scan(root).returncode,0)
   private='undocumented-private-value-should-never-appear'
   sample.write_text('GOOGLE_CLIENT_SECRET='+private+'\n')
   subprocess.run(['git','-C',str(root),'add','.env.production.example'],check=True)
   result=self.scan(root);self.assertNotEqual(result.returncode,0)
   self.assertNotIn(private,result.stdout+result.stderr)
 def test_real_environment_filename_is_rejected_even_with_no_secret_pattern(self):
  with tempfile.TemporaryDirectory() as tmp:
   root=Path(tmp);subprocess.run(['git','init','-q',str(root)],check=True)
   (root/'.env.local').write_text('FLAG=true\n');subprocess.run(['git','-C',str(root),'add','.env.local'],check=True)
   self.assertNotEqual(self.scan(root).returncode,0)
