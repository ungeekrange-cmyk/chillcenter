#!/usr/bin/env python3
"""Focused filesystem/systemctl fakes for the Bonjour-family operator."""
import importlib.util
import json
import os
import pathlib
import tempfile
import unittest
from types import SimpleNamespace
from unittest import mock

SOURCE = pathlib.Path(__file__).with_name("pi-bonjour-family-operator.py")
spec = importlib.util.spec_from_file_location("bonjour", SOURCE)
op = importlib.util.module_from_spec(spec); spec.loader.exec_module(op)


class BonjourFamilyOperatorTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.temp.name)
        self.rollback = self.root / "rollback"; self.rollback.mkdir(mode=0o700); os.chmod(self.rollback, 0o700)
        self.dropin = self.root / "etc" / "systemd" / "system" / "chillcenter.service.d" / op.DROPIN_NAME
        self.families = set(op.BASE_FAMILIES)

    def tearDown(self):
        self.temp.cleanup()

    def root_stat(self):
        real_stat, real_lstat = os.stat, os.lstat
        def rooted(fn):
            def inspect(*args, **kwargs):
                source = fn(*args, **kwargs)
                fields = {name: getattr(source, name) for name in dir(source) if name.startswith("st_")}
                fields["st_uid"] = 0
                return SimpleNamespace(**fields)
            return inspect
        return mock.patch.object(op.os, "stat", side_effect=rooted(real_stat)), mock.patch.object(op.os, "lstat", side_effect=rooted(real_lstat))

    def fake_command(self, *argv, check=True):
        if argv[1:3] == ("show", "--property=RestrictAddressFamilies"):
            return SimpleNamespace(stdout=" ".join(sorted(self.families)) + "\n", returncode=0)
        if argv[1] == "cat":
            return SimpleNamespace(stdout="# vendor unit\n[Service]\n", returncode=0)
        if argv[1] == "daemon-reload":
            if self.dropin.exists(): self.families.add("AF_NETLINK")
            else: self.families.discard("AF_NETLINK")
            return SimpleNamespace(stdout="", returncode=0)
        if argv[1] == "is-active": return SimpleNamespace(stdout="active\n", returncode=0)
        if argv[1] == "restart": return SimpleNamespace(stdout="", returncode=0)
        raise AssertionError(argv)

    def test_apply_preserves_private_snapshots_and_adds_only_netlink(self):
        patches = self.root_stat()
        with patches[0], patches[1], mock.patch.object(op, "command", side_effect=self.fake_command), mock.patch.object(op, "dropin_path", return_value=self.dropin):
            op.apply(self.rollback, self.dropin)
        journal = json.loads((self.rollback / op.JOURNAL).read_text())
        self.assertEqual(journal["state"], "applied")
        self.assertEqual(self.dropin.read_bytes(), op.DROPIN_BYTES)
        self.assertEqual(self.families, set(op.BASE_FAMILIES) | {"AF_NETLINK"})
        self.assertTrue((self.rollback / (journal["operation_id"] + "-original-unit.txt")).exists())
        self.assertTrue((self.rollback / (journal["operation_id"] + "-installed-dropin.conf")).exists())

    def test_apply_refuses_existing_owned_path_without_clobbering_it(self):
        self.dropin.parent.mkdir(parents=True); self.dropin.write_text("foreign")
        patches = self.root_stat()
        with patches[0], patches[1], self.assertRaisesRegex(op.Refusal, "already_exists"):
            op.apply(self.rollback, self.dropin)
        self.assertEqual(self.dropin.read_text(), "foreign")

    def test_apply_never_clobbers_an_existing_transition_journal(self):
        journal = self.rollback / op.JOURNAL
        journal.write_text("existing-evidence")
        os.chmod(journal, 0o600)
        patches = self.root_stat()
        with patches[0], patches[1], self.assertRaisesRegex(op.Refusal, "unfinished_transition"):
            op.apply(self.rollback, self.dropin)
        self.assertEqual(journal.read_text(), "existing-evidence")

    def test_apply_refuses_unexpected_family_baseline_before_writing(self):
        self.families.add("AF_PACKET")
        patches = self.root_stat()
        with patches[0], patches[1], mock.patch.object(op, "effective_families", return_value=frozenset(self.families)):
            with self.assertRaisesRegex(op.Refusal, "unexpected_preexisting"):
                op.apply(self.rollback, self.dropin)
        self.assertFalse(self.dropin.exists())

    def test_rollback_removes_only_exact_owned_dropin_and_restarts(self):
        patches = self.root_stat()
        with patches[0], patches[1], mock.patch.object(op, "command", side_effect=self.fake_command), mock.patch.object(op, "dropin_path", return_value=self.dropin):
            op.apply(self.rollback, self.dropin)
            op.rollback(self.rollback, self.dropin)
        self.assertFalse(self.dropin.exists())
        self.assertEqual(self.families, set(op.BASE_FAMILIES))
        self.assertEqual(json.loads((self.rollback / op.JOURNAL).read_text())["state"], "rollback_complete")

    def test_rollback_refuses_modified_dropin_without_removing_it(self):
        patches = self.root_stat()
        with patches[0], patches[1], mock.patch.object(op, "command", side_effect=self.fake_command), mock.patch.object(op, "dropin_path", return_value=self.dropin):
            op.apply(self.rollback, self.dropin)
            self.dropin.write_text("[Service]\nRestrictAddressFamilies=AF_PACKET\n")
            with self.assertRaisesRegex(op.Refusal, "hash_changed"):
                op.rollback(self.rollback, self.dropin)
        self.assertTrue(self.dropin.exists())

    def test_failed_restart_leaves_pending_journal_for_a_safe_retry(self):
        patches = self.root_stat()
        with patches[0], patches[1], mock.patch.object(op, "command", side_effect=self.fake_command), mock.patch.object(op, "dropin_path", return_value=self.dropin):
            op.apply(self.rollback, self.dropin)
        def restart_failure(*argv, **_kwargs):
            if argv[1] == "daemon-reload": return SimpleNamespace(stdout="", returncode=0)
            if argv[1] == "cat": return SimpleNamespace(stdout="# vendor unit\n[Service]\n", returncode=0)
            if argv[1] == "restart": raise op.Refusal("restart_failed")
            raise AssertionError(argv)
        with patches[0], patches[1], mock.patch.object(op, "command", side_effect=restart_failure):
            with self.assertRaisesRegex(op.Refusal, "restart_failed"):
                op.rollback(self.rollback, self.dropin)
        self.assertFalse(self.dropin.exists())
        self.assertEqual(json.loads((self.rollback / op.JOURNAL).read_text())["state"], "rollback_pending")

    def test_rollback_refuses_unit_bytes_changed_before_restart(self):
        patches = self.root_stat()
        with patches[0], patches[1], mock.patch.object(op, "command", side_effect=self.fake_command), mock.patch.object(op, "dropin_path", return_value=self.dropin):
            op.apply(self.rollback, self.dropin)
        def changed_unit(*argv, **_kwargs):
            if argv[1] == "daemon-reload": return SimpleNamespace(stdout="", returncode=0)
            if argv[1] == "cat": return SimpleNamespace(stdout="changed-unit", returncode=0)
            raise AssertionError(argv)
        with patches[0], patches[1], mock.patch.object(op, "command", side_effect=changed_unit):
            with self.assertRaisesRegex(op.Refusal, "original_unit_hash_unproven"):
                op.rollback(self.rollback, self.dropin)
        self.assertFalse(self.dropin.exists())
        self.assertEqual(json.loads((self.rollback / op.JOURNAL).read_text())["state"], "rollback_pending")

    def test_prepared_journal_survives_failed_apply_and_can_be_rolled_back(self):
        patches = self.root_stat()
        def reload_failure(*argv, **_kwargs):
            if argv[1] == "cat": return SimpleNamespace(stdout="# vendor unit\n[Service]\n", returncode=0)
            if argv[1] == "daemon-reload": raise op.Refusal("daemon_reload_failed")
            raise AssertionError(argv)
        with patches[0], patches[1], mock.patch.object(op, "command", side_effect=reload_failure), \
             mock.patch.object(op, "effective_families", return_value=op.BASE_FAMILIES):
            with self.assertRaisesRegex(op.Refusal, "daemon_reload_failed"):
                op.apply(self.rollback, self.dropin)
        # The prepared record was written before the attempted unit write.
        journal = json.loads((self.rollback / op.JOURNAL).read_text())
        self.assertEqual(journal["state"], "prepared")
        with patches[0], patches[1], mock.patch.object(op, "command", side_effect=self.fake_command):
            op.rollback(self.rollback, self.dropin)
        self.assertEqual(json.loads((self.rollback / op.JOURNAL).read_text())["state"], "rollback_complete")


if __name__ == "__main__":
    unittest.main(verbosity=2)
