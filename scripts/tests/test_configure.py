import importlib.util,json,tempfile,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location("configure",Path(__file__).resolve().parents[1]/"configure.py")
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
class ConfigureTests(unittest.TestCase):
 def test_fresh_project_renames_public_identity_without_touching_environment(self):
  with tempfile.TemporaryDirectory() as directory:
   root=Path(directory);(root/"Dioxus.toml").write_text('[web.app]\ntitle = "Old"\n');(root/".env.local").write_text("PRIVATE_CONFIGURATION=untouched\n")
   module.configure(root,'My "New" App','my-new-app')
   self.assertEqual(json.loads((root/"app.json").read_text()),{"name":'My "New" App',"slug":"my-new-app"})
   import tomllib
   self.assertEqual(tomllib.loads((root/"Dioxus.toml").read_text())["web"]["app"]["title"],'My "New" App')
   self.assertEqual((root/".env.local").read_text(),"PRIVATE_CONFIGURATION=untouched\n")
 def test_invalid_manifest_does_not_partially_replace_identity(self):
  with tempfile.TemporaryDirectory() as directory:
   root=Path(directory);(root/"Dioxus.toml").write_text('[web.app]\n');(root/"app.json").write_text('{"name":"Original"}')
   with self.assertRaises(ValueError):module.configure(root,"New App","new-app")
   self.assertEqual((root/"app.json").read_text(),'{"name":"Original"}')
 def test_invalid_names_and_path_like_slugs_cannot_write_files(self):
  for name,slug in [("", "my-app"),("bad\nname","my-app"),("App","../other"),("App","UPPER"),("App","a"),("App","app-")]:
   with self.subTest(name=name,slug=slug),tempfile.TemporaryDirectory() as directory:
    root=Path(directory)
    with self.assertRaises(ValueError):module.configure(root,name,slug)
    self.assertEqual(list(root.iterdir()),[])
if __name__=="__main__":unittest.main()
