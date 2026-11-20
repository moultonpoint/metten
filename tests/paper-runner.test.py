#!/usr/bin/env python3
import json
from pathlib import Path
import plistlib
import runpy
import shutil
import tempfile
import unittest

RUNNER = runpy.run_path(str(Path(__file__).resolve().parents[1] / "scripts/verify-paper.py"))


class PaperRunner(unittest.TestCase):
    def bundle(self, code):
        directory = tempfile.TemporaryDirectory(prefix="metten-paper-runner-test-")
        self.addCleanup(directory.cleanup)
        app = Path(directory.name) / "Test.app"
        binary = app / "Contents/MacOS/ActualExecutable"
        binary.parent.mkdir(parents=True)
        with (app / "Contents/Info.plist").open("wb") as stream:
            plistlib.dump({"CFBundleExecutable": "ActualExecutable"}, stream)
        binary.write_text("#!/usr/bin/env python3\n# metten-paper-check-v1\n" + code)
        binary.chmod(0o700)
        return app

    def test_metadata_executable_success_and_sqlite_receipts_then_cleanup(self):
        app = self.bundle('''import json, sqlite3, sys
from pathlib import Path
workspace=Path(sys.argv[2])
db=sqlite3.connect(workspace/'session.sqlite')
db.executescript("PRAGMA user_version=4; CREATE TABLE paper_runs(run_id INTEGER,cash INTEGER); INSERT INTO paper_runs VALUES(1,10736870000),(2,10000000000); CREATE TABLE paper_settlements(payout INTEGER,net INTEGER); INSERT INTO paper_settlements VALUES(1000000000,736870000); CREATE TABLE paper_attempts(attempt_id INTEGER,state TEXT,filled_quantity INTEGER,cost INTEGER,fee INTEGER); INSERT INTO paper_attempts VALUES(1,'full',10,250000000,13130000),(2,'zero',0,0,0);")
db.close()
print(json.dumps(dict(protocol='metten-paper-check-v1',kind='paper_check',origin='synthetic_controlled',workspace=str(workspace))))
''')
        result = RUNNER["run"](app)
        self.assertEqual(result.code, 0, result.output)
        self.assertFalse(Path(json.loads(result.output)["workspace"]).exists())
        self.assertIsNone(result.process)

    def test_failed_child_exits_without_leaving_owned_resources(self):
        app = self.bundle("import sys\nprint('controlled failure')\nsys.exit(3)\n")
        result = RUNNER["run"](app)
        self.assertEqual(result.code, 1)
        self.assertIn("exit 3", result.output)
        self.assertIsNone(result.workspace)

    def test_timeout_sends_stop_and_waits_for_cooperative_exit(self):
        app = self.bundle("import sys\nassert sys.stdin.readline() == 'STOP\\n'\nprint('cooperatively stopped')\n")
        result = RUNNER["run"](app, timeout=0.1, grace=2)
        self.assertEqual(result.code, 1)
        self.assertIn("timed out", result.output)
        self.assertIn("cooperatively stopped", result.output)
        self.assertIsNone(result.process)
        self.assertIsNone(result.workspace)

    def test_unconfirmed_exit_preserves_open_database_until_actual_exit(self):
        app = self.bundle('''import sqlite3, sys, time
from pathlib import Path
workspace=Path(sys.argv[2]); gate=Path(__file__).parent/'release'
db=sqlite3.connect(workspace/'session.sqlite'); db.execute('CREATE TABLE held(x)'); db.commit()
start=time.monotonic()
while not gate.exists() and time.monotonic()-start<10: time.sleep(.01)
db.close()
''')
        result = RUNNER["run"](app, timeout=1, grace=0.2)
        try:
            self.assertEqual(result.code, 1)
            self.assertIsNotNone(result.process)
            self.assertIsNone(result.process.poll())
            self.assertTrue(result.workspace.is_dir())
            self.assertTrue((result.workspace / "session.sqlite").is_file())
            self.assertIn("Preserved", result.output)
        finally:
            (app / "Contents/MacOS/release").touch()
            if result.process is not None:
                result.process.wait(timeout=5)
                result.process.stdin.close()
                result.process.stdout.close()
            if result.workspace is not None:
                shutil.rmtree(result.workspace)

    def test_unsupported_bundle_is_refused_before_launch(self):
        app = self.bundle("raise RuntimeError('must not launch')\n")
        binary = app / "Contents/MacOS/ActualExecutable"
        binary.write_text("#!/usr/bin/env python3\nraise RuntimeError('must not launch')\n")
        with self.assertRaisesRegex(ValueError, "protocol marker"):
            RUNNER["run"](app)

    def test_missing_owned_directory_is_reported_after_confirmed_exit(self):
        app = self.bundle("import shutil, sys\nshutil.rmtree(sys.argv[2])\nprint('{}')\n")
        result = RUNNER["run"](app)
        self.assertEqual(result.code, 1)
        self.assertIn("Paper runner failed", result.output)
        self.assertIsNone(result.process)

    def test_noisy_child_has_bounded_output(self):
        app = self.bundle("print('x' * 100000)\n")
        result = RUNNER["run"](app)
        self.assertEqual(result.code, 1)
        self.assertIn("exceeded its bound", result.output)
        self.assertLess(len(result.output), RUNNER["OUTPUT_LIMIT"] + 200)


if __name__ == "__main__":
    unittest.main()
