
use fluent_resmgr::resource_manager::ResourceManager;
use sys_locale::get_locale;
use unic_langid::LanguageIdentifier;

pub fn translate(change: &str) -> String{
    let locale = get_locale().unwrap_or_else(|| String::from("en-US"));

    let langid: LanguageIdentifier = locale.parse().expect("wrong language"); 
    let locales = vec![langid.into()];
    let resources = vec!["message.ftl".into()];
    let mgr = ResourceManager::new("./translations/{locale}/{res_id}".into()); 
    let bundle = mgr.get_bundle(locales, resources);
    let value = bundle.get_message(change).expect("Message doesn't exist.");
    let pattern = value.value().expect("Message has no value."); 

    let mut errors = vec![];
    let msg = bundle.format_pattern(&pattern, None, &mut errors).to_string();
    msg
}
