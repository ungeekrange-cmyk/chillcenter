#!/usr/bin/env python3
import argparse, hashlib, importlib.util, json, os, pathlib, tempfile, unittest
from contextlib import ExitStack
from types import SimpleNamespace
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
  op.atomic_json(self.j,{"state":state,"service":"fixture","binary":str(self.bin),"old_copy":str(self.old),"new_stage":str(self.new),"old_sha":digest(b"old"),"new_sha":digest(b"new"),"maintenance_nonce":"a"*32})
 def root_filesystem(self):
  # Real temporary files/modes/links/fsync, simulated UID only; never /run.
  stack=ExitStack(); real_stat,real_lstat=os.stat,os.lstat
  def rooted(fn):
   def inspect(*args,**kwargs):
    s=fn(*args,**kwargs)
    fields={k:getattr(s,k) for k in dir(s) if k.startswith("st_")}; fields["st_uid"]=0
    return SimpleNamespace(**fields)
   return inspect
  stack.enter_context(mock.patch.object(op,"MAINTENANCE_PATH",self.root/"runtime"/"maintenance"))
  stack.enter_context(mock.patch.object(op.os,"stat",side_effect=rooted(real_stat)))
  stack.enter_context(mock.patch.object(op.os,"lstat",side_effect=rooted(real_lstat)))
  stack.enter_context(mock.patch.object(op.os,"geteuid",return_value=0))
  stack.enter_context(mock.patch.object(op.os,"chown"))
  return stack
 def deploy_arguments(self):
  return ["operator","deploy","--base-url","https://fixture","--service","fixture",
   "--binary",str(self.bin),"--candidate",str(self.new),"--config",str(self.old),
   "--cert",str(self.old),"--key",str(self.old),"--data-dir",str(self.root),
   "--web-dir",str(self.root),"--rollback-dir",str(self.rb),
   "--old-sha",digest(b"old"),"--new-sha",digest(b"new")]
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
 def test_backup_returns_receipt_path_and_validated_baseline(self):
  args=argparse.Namespace(service_user="fixture",binary=self.bin,config=self.old,data_dir=self.root)
  proc=mock.Mock(returncode=0); proc.communicate.return_value=("Sauvegarde terminée : /private/snapshot\n","")
  baseline={"schema":3,"digest":"x","tables":[]}
  with mock.patch.object(op.subprocess,"Popen",return_value=proc), mock.patch.object(op,"validate_backup",return_value=baseline):
   receipt,got=op.backup(args)
  self.assertEqual((receipt,got),(pathlib.Path("/private/snapshot"),baseline))
 def test_durable_replace_fsyncs_both_parent_directories(self):
  src=self.rb/"source"; dst=self.root/"destination"; src.write_bytes(b"x"); seen=[]
  with mock.patch.object(op.os,"fsync",side_effect=lambda fd:seen.append(fd)):
   op.durable_replace(src,dst)
  self.assertEqual(dst.read_bytes(),b"x"); self.assertEqual(len(seen),2)
 def test_recover_stops_new_service_before_restoring_external_binary(self):
  self.bin.write_bytes(b"new"); self.receipt("swapped"); calls=[]
  args=argparse.Namespace(rollback_dir=self.rb,service="fixture",base_url="https://fixture",cert=self.old)
  with mock.patch.object(op,"validate_recovery",return_value=op.journal_load(self.j)), mock.patch.object(op,"owned_maintenance",return_value=None), mock.patch.object(op,"old_health"), mock.patch.object(op,"active",side_effect=[True]), mock.patch.object(op,"service",side_effect=lambda _s,a:calls.append(a)):
   op.recover(args)
  self.assertEqual(calls,["stop","start"]); self.assertEqual(self.bin.read_bytes(),b"old")
 def test_quiescing_abort_never_stops_active_old_service(self):
  self.receipt("quiescing"); args=argparse.Namespace(rollback_dir=self.rb,service="fixture",base_url="https://fixture",cert=self.old)
  with mock.patch.object(op,"validate_recovery",return_value=op.journal_load(self.j)), mock.patch.object(op,"owned_maintenance",return_value=None), mock.patch.object(op,"old_health"), mock.patch.object(op,"active",return_value=True), mock.patch.object(op,"service") as svc:
   op.recover(args)
  svc.assert_not_called()
 def test_recover_restores_missing_binary_only_from_pinned_old_copy(self):
  self.bin.unlink(); self.receipt("stopping")
  args=argparse.Namespace(rollback_dir=self.rb,service="fixture",base_url="https://fixture",cert=self.old)
  with mock.patch.object(op,"validate_recovery",return_value=op.journal_load(self.j)), mock.patch.object(op,"owned_maintenance",return_value=None), mock.patch.object(op,"old_health"), mock.patch.object(op,"active",side_effect=[False]), mock.patch.object(op,"service"):
   op.recover(args)
  self.assertEqual(self.bin.read_bytes(),b"old")
 def test_recover_does_not_overwrite_unknown_binary(self):
  self.bin.write_bytes(b"unknown"); self.receipt("swapped")
  args=argparse.Namespace(rollback_dir=self.rb,service="fixture",base_url="https://fixture",cert=self.old)
  with mock.patch.object(op,"validate_recovery",return_value=op.journal_load(self.j)), mock.patch.object(op,"owned_maintenance",return_value=None), mock.patch.object(op,"active",return_value=False), mock.patch.object(op,"service"):
   with self.assertRaises(op.Refusal): op.recover(args)
  self.assertEqual(self.bin.read_bytes(),b"unknown")
 def test_real_maintenance_helpers_survive_restrictive_umask(self):
  with self.root_filesystem():
   previous=os.umask(0o077)
   try: token=op.acquire_maintenance(None,"a"*32)
   finally: os.umask(previous)
   self.assertEqual(op.MAINTENANCE_PATH.parent.stat().st_mode & 0o777,0o755)
   self.assertEqual(op.MAINTENANCE_PATH.stat().st_mode & 0o777,0o600)
   self.assertEqual(op.owned_maintenance("a"*32),token)
   op.release_maintenance(token)
   self.assertFalse(op.MAINTENANCE_PATH.exists())
 def test_existing_writable_maintenance_parent_is_rejected(self):
  with self.root_filesystem():
   op.MAINTENANCE_PATH.parent.mkdir(); os.chmod(op.MAINTENANCE_PATH.parent,0o777)
   with self.assertRaisesRegex(op.Refusal,"maintenance_parent_invalid"): op.acquire_maintenance(None,"a"*32)
   self.assertFalse(op.MAINTENANCE_PATH.exists())
 def test_foreign_nonce_is_never_removed(self):
  with self.root_filesystem():
   token=op.acquire_maintenance(None,"a"*32)
   with self.assertRaises(op.Refusal): op.owned_maintenance("b"*32)
   with self.assertRaises(op.Refusal): op.release_maintenance((*token[:2],"b"*32))
   self.assertTrue(op.MAINTENANCE_PATH.exists()); op.release_maintenance(token)
 def test_published_flag_is_recoverable_from_its_journal_nonce(self):
  self.receipt("quiescing")
  args=argparse.Namespace(binary=self.bin,config=self.old,cert=self.old,key=self.old,
   data_dir=self.root,web_dir=self.root,rollback_dir=self.rb,service="fixture",
   old_sha=digest(b"old"),new_sha=digest(b"new"))
  with self.root_filesystem(),mock.patch.object(op,"old_health"),mock.patch.object(op,"service") as service:
   op.acquire_maintenance(None,"a"*32); op.recover(args)
   self.assertFalse(op.MAINTENANCE_PATH.exists()); service.assert_not_called()
  self.assertEqual(op.journal_load(self.j)["state"],"rollback_complete")
 def test_actual_quiescence_helper_requires_matching_fresh_generation(self):
  with self.root_filesystem():
   token=op.acquire_maintenance(None,"a"*32); proof=self.root/"updates"; proof.mkdir()
   (proof/"quiescence.json").write_text(json.dumps({"updated_unix":int(op.time.time()),
    "maintenance_generation_ns":str(op.MAINTENANCE_PATH.stat().st_mtime_ns),
    "quiescing":True,"busy":False,"active_mutations":0}))
   args=argparse.Namespace(data_dir=self.root,base_url="https://fixture",cert=self.old)
   with mock.patch.object(op,"strict_json",return_value={"busy":False,"active_mutations":0,"quiescing":True}): op.quiescent(args,token)
   op.release_maintenance(token)
 def test_true_main_success_swaps_only_binary_and_releases_owned_flag(self):
  baseline={"schema":3,"digest":"fixture","tables":[]}; calls=[]
  with self.root_filesystem(),mock.patch.object(op.sys,"argv",self.deploy_arguments()),\
   mock.patch.object(op,"quiet"),mock.patch.object(op,"elf_aarch64"),\
   mock.patch.object(op,"snapshot_db",return_value=baseline),mock.patch.object(op,"invariant",return_value={"fixture":"unchanged"}),\
   mock.patch.object(op,"quiescent"),mock.patch.object(op,"backup",return_value=(self.root/"backup",baseline)),\
   mock.patch.object(op,"service",side_effect=lambda _s,a:calls.append(a)),\
   mock.patch.object(op,"active",side_effect=[False,True]),mock.patch.object(op,"v071"):
   op.main(); self.assertFalse(op.MAINTENANCE_PATH.exists())
  self.assertEqual(calls,["stop","start"]); self.assertEqual(self.bin.read_bytes(),b"new")
  self.assertEqual(op.journal_load(self.j)["state"],"complete")
 def test_failed_rollback_health_keeps_maintenance_and_unresolved_receipt(self):
  baseline={"schema":3,"digest":"fixture","tables":[]}
  with self.root_filesystem(),mock.patch.object(op.sys,"argv",self.deploy_arguments()),\
   mock.patch.object(op,"quiet"),mock.patch.object(op,"elf_aarch64"),\
   mock.patch.object(op,"snapshot_db",return_value=baseline),mock.patch.object(op,"invariant",return_value={}),\
   mock.patch.object(op,"quiescent"),mock.patch.object(op,"backup",side_effect=op.Refusal("backup_failed")),\
   mock.patch.object(op,"service"),mock.patch.object(op,"active",return_value=False),\
   mock.patch.object(op,"recover",side_effect=op.Refusal("rollback_health_failed")):
   with self.assertRaisesRegex(op.Refusal,"rollback_health_failed"): op.main()
   self.assertTrue(op.MAINTENANCE_PATH.exists())
   self.assertEqual(op.journal_load(self.j)["state"],"stopping")
if __name__=="__main__": unittest.main(verbosity=2)
