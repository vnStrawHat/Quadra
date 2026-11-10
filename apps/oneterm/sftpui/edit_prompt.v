// What happens after a save: the Upload prompt, and ending edit sessions.
module sftpui

import os
import ssh
import quadra.core
import quadra.kit
import quadra.ui

fn (mut p SftpPanel) on_saved(mut s EditSession, mut cx core.ViewCx[SftpPanel]) {
	if s.uploading || s.prompt {
		s.pending = true
		return
	}
	if s.always {
		p.begin_upload_edit(s.id, mut cx)
		return
	}
	s.prompt = true
	p.bind(mut cx)
	p.prompt_upload(mut s, mut cx)
}

// prompt_upload asks to upload the saved copy, with the per-session Always box.
fn (mut p SftpPanel) prompt_upload(mut s EditSession, mut cx core.ViewCx[SftpPanel]) {
	name := ssh.remote_file_name(s.remote)
	mut checked := &ModalCheck{}
	id := s.id
	mut m := &ModalRef{}
	ent := cx.entity
	m.id = kit.open_modal(mut cx.Cx, kit.ModalOptions{
		title:  'Upload change'
		width:  440
		body:   fn [name, checked, ent] () core.Element {
			return ui.div().flex_col().gap(10).w_full().child(ui.text('"${name}" was saved. Upload it to the remote host?')).child(kit.checkbox('sftp-edit-always',
				checked.on).label('Always upload this file while editing').on_change(core.listener_with[SftpPanel, CheckRef, bool](ent, CheckRef{checked}, fn (mut _ SftpPanel, c CheckRef, v &bool, mut cx core.ViewCx[SftpPanel]) {
				mut mc := unsafe { c.c }
				mc.on = *v
				cx.notify()
			})))
		}
		footer: fn [ent, m, id, checked] () core.Element {
			return ui.div().flex_row().gap(4).child(kit.button('upload-cancel').label('Cancel').on_click(core.listener_with[SftpPanel, UploadAnswer, core.ClickEvent](ent, UploadAnswer{m, id, checked, false},
				answer_upload))).child(kit.button('upload-ok').label('Upload').primary().on_click(core.listener_with[SftpPanel, UploadAnswer, core.ClickEvent](ent, UploadAnswer{m, id, checked, true},
				answer_upload)))
		}
	})
}

struct CheckRef {
	c &ModalCheck = unsafe { nil }
}

@[heap]
struct ModalCheck {
mut:
	on bool
}

struct UploadAnswer {
	modal   &ModalRef = unsafe { nil }
	id      u64
	checked &ModalCheck = unsafe { nil }
	yes     bool
}

fn answer_upload(mut p SftpPanel, a UploadAnswer, _ &core.ClickEvent, mut cx core.ViewCx[SftpPanel]) {
	kit.close_modal(mut cx.Cx, a.modal.id)
	mut s := p.edits[a.id] or { return }
	s.prompt = false
	if !a.yes {
		return
	}
	s.always = a.checked.on
	p.begin_upload_edit(a.id, mut cx)
}

fn (mut p SftpPanel) end_edit(id u64) {
	s := p.edits[id] or { return }
	os.rmdir_all(os.dir(s.temp)) or {}
	p.edits.delete(id)
	os.rmdir(pid_dir(p.edit_dir, os.getpid())) or {} // removes only an empty folder
}

fn (mut p SftpPanel) end_edits_of(key u64) {
	for id, s in p.edits.clone() {
		if s.key == key {
			p.end_edit(id)
		}
	}
}
