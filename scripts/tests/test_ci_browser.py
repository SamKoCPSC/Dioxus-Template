import importlib.util
from pathlib import Path
from types import SimpleNamespace
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('ci_browser', Path(__file__).resolve().parents[1] / 'ci-browser.py')
ci = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci)


class BrowserRunnerTests(unittest.TestCase):
    def test_failed_smoke_still_removes_its_owned_instance(self):
        with tempfile.TemporaryDirectory() as directory:
            state = Path(directory) / 'state'
            def run(command, **kwargs):
                if 'up' in command:
                    state.mkdir()
                if '--showcase' in command:
                    raise subprocess.CalledProcessError(7, command)
                return subprocess.CompletedProcess(command, 0)
            with patch.object(ci.host, 'state_path', return_value=state), patch.object(ci.subprocess, 'run', side_effect=run) as invoked:
                result = ci.run(SimpleNamespace(output=Path(directory)/'artifacts', dx=None, managed=False, chromium=None))
            self.assertEqual(result, 7)
            self.assertIn('down', invoked.call_args.args[0])
            instance = invoked.call_args.args[0][-1]
            self.assertTrue(instance.startswith('browser-ci-'))

    def test_preexisting_instance_is_never_claimed_or_torn_down(self):
        with tempfile.TemporaryDirectory() as directory:
            state = Path(directory)
            with patch.object(ci.host, 'state_path', return_value=state), patch.object(ci.subprocess, 'run') as invoked:
                with self.assertRaises(ValueError):
                    ci.run(SimpleNamespace(output=state/'artifacts', dx=None, managed=False, chromium=None))
            invoked.assert_not_called()

    def test_artifacts_cannot_be_written_in_the_checkout(self):
        with patch.object(ci.subprocess, 'run') as invoked:
            with self.assertRaises(ValueError):
                ci.run(SimpleNamespace(output=ci.ROOT/'captures'))
        invoked.assert_not_called()


if __name__ == '__main__':
    unittest.main()
