use fluent_resmgr::resource_manager::ResourceManager;
use once_cell::sync::OnceCell;
use sys_locale::get_locale;
use unic_langid::LanguageIdentifier;

pub fn translate(message: &str) -> String {
    let locale = get_locale().unwrap_or_else(|| String::from("en-US"));
    let langid: LanguageIdentifier = locale.parse().expect("wrong language");
    let locales = vec![langid];
    let resources = vec!["message.ftl".into()];

    let mgr_cell = OnceCell::new();
    let mgr = mgr_cell.get_or_init(|| {
        ResourceManager::new("/usr/share/fd/translations/{locale}/{res_id}".into())
    });

    let bundle_cell = OnceCell::new();
    let bundle = bundle_cell.get_or_init(|| mgr.get_bundle(locales, resources));

    let value = bundle.get_message(message).expect("Message doesn't exist.");
    let pattern = value.value().expect("Message has no value.");

    let mut errors = vec![];
    let msg = bundle.format_pattern(pattern, None, &mut errors);
    msg.to_string()
}
