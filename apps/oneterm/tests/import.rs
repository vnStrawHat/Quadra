//! Taking over from OneTerm (specs/19, "Taking over from OneTerm"): its saved sessions become
//! hosts with their colours, its terminal's settings are taken at the first pass alone, and a
//! host 1Term has is left as it is. The folder is the test's, under `.tmp`.

use std::rc::Rc;

use oneterm::{NewSession, OneTerm};
use quadra::{Settings, provide};
use quadra_term::echo;
use quadra_testing::Harness;

#[test]
fn hosts_and_the_terminal_s_settings_are_taken_over_and_a_host_there_is_left() {
    let folder = std::path::absolute(".tmp/import/.OneTerm").unwrap();
    std::fs::create_dir_all(&folder).unwrap();
    let saved = r##"{"schema_version":2,"sessions":[
        {"id":1,"label":"prod","host":"10.0.0.1","port":22,"username":"deploy","group":"infra",
         "color":"#56B6C2","sealed_password":"otv1:AAEC"},
        {"id":2,"label":"lab","host":"10.0.0.9","port":2222,"username":"me","jump_host":1}]}"##;
    std::fs::write(folder.join("ssh_session.json"), saved).unwrap();
    let terminal = r#"{"font":{"family":"Legacy Mono","size":15.0},"scroll":{"scrollback_history":20000}}"#;
    std::fs::write(folder.join("terminal.json"), terminal).unwrap();

    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 700.0);
    let settings = Settings::memory();
    let (given, sessions): (_, NewSession) = (settings.clone(), Rc::new(|_| echo()));
    let model = Rc::new(std::cell::RefCell::new(None));
    let slot = model.clone();
    h.mount(move || {
        provide(given.clone());
        let app = OneTerm::new(sessions);
        *slot.borrow_mut() = Some(app.model.clone());
        app.view()
    });
    h.settle();
    let model = model.borrow_mut().take().unwrap();

    assert_eq!(model.take_over(&folder, true), (2, 1));
    h.settle();
    let rows = settings.value("ssh.hosts", Vec::<Vec<String>>::new()).get();
    assert_eq!(rows[0][..4], ["prod", "infra", "deploy@10.0.0.1", "Password"].map(String::from));
    assert_eq!(rows[0].get(7).map(String::as_str), Some("#56B6C2"), "{rows:?}");
    assert_eq!((rows[1][2].as_str(), rows[1][5].as_str()), ("me@10.0.0.9:2222", "prod"));
    assert_eq!(settings.value("terminal.font_family", String::new()).get(), "Legacy Mono");
    assert_eq!(settings.value("terminal.scrollback", 0.0).get(), 20000.0);
    // The rail lists them, in their group.
    assert!(h.has_text("prod") && h.has_text("lab"), "{}", h.semantics());

    // Again: nothing new, and a setting changed since stays.
    settings.value("terminal.font_family", String::new()).set("Mine".into());
    assert_eq!(model.take_over(&folder, false), (0, 0));
    assert_eq!(settings.value("ssh.hosts", Vec::<Vec<String>>::new()).get().len(), 2);
    assert_eq!(settings.value("terminal.font_family", String::new()).get(), "Mine");
}
