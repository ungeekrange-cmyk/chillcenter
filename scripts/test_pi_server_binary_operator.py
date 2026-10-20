#!/usr/bin/env python3
import hashlib, importlib.util, json, os, pathlib, tempfile, unittest

SOURCE=pathlib.Path(__file__).with_name("pi-server-binary-operator.py")
spec=importlib.util.spec_from_file_location("op",SOURCE); op=importlib.util.module_from_spec(spec); spec.loader.exec_module(op)
def digest(b): return hashlib.sha256(b).hexdigest()

class Guards(unittest.TestCase):
 def setUp(self):
  self.t=tempfile.TemporaryDirectory(); self.root=pathlib.Path(self.t.name); self.rb=self.root/"rollback"; self.rb.mkdir(mode=0o700); os.chmod(self.rb,0o700)
  self.old=self.rb/"old"; self.new=self.rb/"new"; self.bin=self.rb/"binary"; self.old.write_bytes(b"old"); self.new.write_bytes(b"new"); self.bin.write_bytes(b"old")
  self.j=self.rb/"binary-transition.json"
 def tearDown(self): self.t.cleanup()
 def receipt(self,state="prepared"):
  op.atomic_json(self.j,{"state":state,"binary":str(self.bin),"old_copy":str(self.old),"new_stage":str(self.new),"old_sha":digest(b"old"),"new_sha":digest(b"new")})
 def test_atomic_journal_roundtrip_and_private_root(self):
  self.receipt(); self.assertEqual(op.journal_load(self.j)["state"],"prepared")
 def test_journal_refuses_paths_outside_private_root(self):
  self.receipt(); d=json.loads(self.j.read_text()); d["binary"]="/tmp/other"; self.j.write_text(json.dumps(d))
  with self.assertRaises(op.Refusal): op.journal_load(self.j)
 def test_journal_refuses_unknown_state(self):
  self.receipt(); d=json.loads(self.j.read_text()); d["state"]="complete"; self.j.write_text(json.dumps(d))
  with self.assertRaises(op.Refusal): op.journal_load(self.j)
 def test_elf_guard_rejects_non_elf_fixture(self):
  with self.assertRaises(op.Refusal): op.elf_aarch64(self.new)
 def test_backup_manifest_rejects_extra_or_tampered_file(self):
  snap=self.rb/"snapshot"; snap.mkdir(mode=0o700); os.chmod(snap,0o700); (snap/"catalog.sqlite").write_bytes(b"x")
  (snap/"manifest.json").write_text(json.dumps({"format_version":1,"schema_version":3,"data_root":"/data","entries":[{"path":"catalog.sqlite","size":1,"sha256":digest(b"x")}]}))
  op.validate_backup(snap,pathlib.Path("/data")); (snap/"unexpected").write_bytes(b"x")
  with self.assertRaises(op.Refusal): op.validate_backup(snap,pathlib.Path("/data"))
if __name__=="__main__": unittest.main(verbosity=2)
