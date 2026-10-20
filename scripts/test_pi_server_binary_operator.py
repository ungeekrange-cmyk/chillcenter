#!/usr/bin/env python3
import argparse, hashlib, importlib.util, json, os, pathlib, tempfile, unittest
from unittest import mock

SOURCE=pathlib.Path(__file__).with_name("pi-server-binary-operator.py")
spec=importlib.util.spec_from_file_location("op",SOURCE); op=importlib.util.module_from_spec(spec); spec.loader.exec_module(op)
def digest(b): return hashlib.sha256(b).hexdigest()

class Guards(unittest.TestCase):
 def setUp(self):
  self.t=tempfile.TemporaryDirectory(); self.root=pathlib.Path(self.t.name); self.rb=self.root/"rollback"; self.rb.mkdir(mode=0o700); os.chmod(self.rb,0o700)
  self.old=self.rb/"old"; self.new=self.rb/"new"; self.bin=self.root/"installed-binary"; self.old.write_bytes(b"old"); self.new.write_bytes(b"new"); self.bin.write_bytes(b"old")
  self.j=self.rb/"binary-transition.json"
 def tearDown(self): self.t.cleanup()
 def receipt(self,state="prepared"):
  op.atomic_json(self.j,{"state":state,"service":"fixture","binary":str(self.bin),"old_copy":str(self.old),"new_stage":str(self.new),"old_sha":digest(b"old"),"new_sha":digest(b"new")})
 def test_atomic_journal_roundtrip_and_private_root(self):
  self.receipt(); self.assertEqual(op.journal_load(self.j)["state"],"prepared")
 def test_journal_allows_installed_binary_outside_rollback_root(self):
  self.receipt()
  self.assertEqual(op.journal_load(self.j)["binary"],str(self.bin))
 def test_journal_refuses_rollback_copy_outside_private_root(self):
  self.receipt(); d=json.loads(self.j.read_text()); d["old_copy"]="/tmp/other"; self.j.write_text(json.dumps(d))
  with self.assertRaises(op.Refusal): op.journal_load(self.j)
 def test_completed_receipt_is_idempotently_accepted(self):
  self.receipt("complete"); self.assertEqual(op.journal_load(self.j)["state"],"complete")
 def test_forged_world_writable_journal_is_refused(self):
  self.receipt(); os.chmod(self.j,0o777)
  with self.assertRaises(op.Refusal): op.journal_load(self.j)
 def test_elf_guard_rejects_non_elf_fixture(self):
  with self.assertRaises(op.Refusal): op.elf_aarch64(self.new)
 def test_backup_manifest_rejects_extra_or_tampered_file(self):
  snap=self.rb/"snapshot"; snap.mkdir(mode=0o700); os.chmod(snap,0o700); (snap/"catalog.sqlite").write_bytes(b"x")
  (snap/"manifest.json").write_text(json.dumps({"format_version":1,"schema_version":3,"data_root":"/data","entries":[{"path":"catalog.sqlite","size":1,"sha256":digest(b"x")}]}))
  with self.assertRaises(op.Refusal): op.validate_backup(snap,pathlib.Path("/data"))
 def test_web_digest_refuses_symlink(self):
  web=self.root/"web"; web.mkdir(); (web/"bad").symlink_to("/etc/passwd")
  with self.assertRaises(op.Refusal): op.web_digest(web)
 def test_recover_stops_new_service_before_restoring_external_binary(self):
  self.bin.write_bytes(b"new"); self.receipt("swapped"); calls=[]
  args=argparse.Namespace(rollback_dir=self.rb,service="fixture",base_url="https://fixture",cert=self.old)
  with mock.patch.object(op,"validate_recovery",return_value=op.journal_load(self.j)), mock.patch.object(op,"strict_json",return_value={}), mock.patch.object(op,"active",side_effect=[True,True]), mock.patch.object(op,"service",side_effect=lambda _s,a:calls.append(a)):
   op.recover(args)
  self.assertEqual(calls,["stop","start"]); self.assertEqual(self.bin.read_bytes(),b"old")
 def test_recover_restores_missing_binary_only_from_pinned_old_copy(self):
  self.bin.unlink(); self.receipt("stopping")
  args=argparse.Namespace(rollback_dir=self.rb,service="fixture",base_url="https://fixture",cert=self.old)
  with mock.patch.object(op,"validate_recovery",return_value=op.journal_load(self.j)), mock.patch.object(op,"strict_json",return_value={}), mock.patch.object(op,"active",side_effect=[False,True]), mock.patch.object(op,"service"):
   op.recover(args)
  self.assertEqual(self.bin.read_bytes(),b"old")
 def test_recover_does_not_overwrite_unknown_binary(self):
  self.bin.write_bytes(b"unknown"); self.receipt("swapped")
  args=argparse.Namespace(rollback_dir=self.rb,service="fixture",base_url="https://fixture",cert=self.old)
  with mock.patch.object(op,"validate_recovery",return_value=op.journal_load(self.j)), mock.patch.object(op,"active",return_value=False), mock.patch.object(op,"service"):
   with self.assertRaises(op.Refusal): op.recover(args)
  self.assertEqual(self.bin.read_bytes(),b"unknown")
if __name__=="__main__": unittest.main(verbosity=2)
