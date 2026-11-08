"""Packaging layout tests; no native packaging executables or network required."""
import contextlib
import importlib.util
import io
from pathlib import Path
import struct
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('package', Path(__file__).with_name('package.py'))
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class PackagingTests(unittest.TestCase):
    def test_portable_archive_and_architecture_guard(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            binary = root / 'purrloader'
            binary.write_bytes(b'\x7fELF' + bytes(14) + struct.pack('<H', 62))
            licenses = root / 'licenses.html'
            licenses.write_text('license fixture')
            args = ['package.py', '--arch', 'x64', '--binary', str(binary), '--licenses', str(licenses), '--format', 'archive', '--output', str(root / 'out')]
            with patch.object(sys, 'argv', args), patch.object(package.platform, 'system', return_value='Linux'):
                package.main()
            with tarfile.open(root / 'out/Purrloader-linux-x64.tar.gz') as archive:
                self.assertEqual(archive.extractfile('Purrloader/usr/bin/purrloader').read(), binary.read_bytes())
                self.assertEqual(archive.extractfile('Purrloader/usr/share/doc/purrloader/THIRD_PARTY_LICENSES.html').read(), b'license fixture')
                self.assertTrue(archive.getmember('Purrloader/purrloader').mode & 0o111)
                self.assertIn(b'usr/bin/purrloader', archive.extractfile('Purrloader/purrloader').read())
            args[args.index('x64')] = 'arm64'
            with patch.object(sys, 'argv', args), patch.object(package.platform, 'system', return_value='Linux'), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit):
                    package.main()
            self.assertFalse((root / 'out/Purrloader-linux-arm64.tar.gz').exists())


if __name__ == '__main__':
    unittest.main()
