import importlib.util,json,stat,subprocess,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('host_dev',Path(__file__).resolve().parents[1]/'host-dev.py')
host=importlib.util.module_from_spec(spec);spec.loader.exec_module(host)
class HostLifecycleTests(unittest.TestCase):
 def test_names_cannot_escape_private_runtime_root(self):
  for name in ['../app','App','a;echo','', 'a'*41]:
   with self.subTest(name=name),self.assertRaises(ValueError):host.state_path(name)
  self.assertTrue(host.state_path('my-app').name.endswith('-my-app'))
 def test_state_is_private_and_foreign_worktree_is_rejected(self):
  with tempfile.TemporaryDirectory() as directory:
   state=Path(directory)/'instance';state.mkdir()
   host.private_write(state/'metadata.json',json.dumps({'instance':state.name,'worktree':'/other'}))
   self.assertEqual(stat.S_IMODE((state/'metadata.json').stat().st_mode),0o600)
   with self.assertRaises(ValueError):host.load_meta(state)
 def test_foreign_container_labels_prevent_destructive_teardown(self):
  meta={'instance':'owned','token':'expected'}
  responses=[subprocess.CompletedProcess([],0,'container\n'),subprocess.CompletedProcess([],0,json.dumps({'com.app.dev-instance':'owned','com.app.dev-token':'foreign'}))]
  with patch.object(host.subprocess,'run',side_effect=responses),patch.object(host,'compose') as compose:
   with self.assertRaises(ValueError):host.cleanup(meta,Path('/unused'),False)
   compose.assert_not_called()
 def test_reused_pid_is_never_signaled(self):
  meta={'pid':123,'process_signature':'original'}
  with patch.object(host,'start_signature',return_value='replacement'),patch.object(host.os,'killpg') as kill:
   with self.assertRaises(ValueError):host.cleanup(meta,Path('/unused'),False)
   kill.assert_not_called()
if __name__=='__main__':unittest.main()
