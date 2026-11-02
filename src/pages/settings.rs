use crate::components::ClientForm;
use crate::{
    auth::{account_binding::use_form_account, context::AuthContext},
    components::{AuthRequired, PageHeading, SettingsNav},
    Route,
};
use dioxus::prelude::*;
#[component]
pub fn Profile() -> Element {
    let auth = crate::auth::context::use_auth();
    rsx! {AuthRequired{SettingsNav{} if let Some(profile)=auth.current_user{ProfileForm{profile}}}}
}
#[component]
fn ProfileForm(profile: crate::auth::context::UserProfile) -> Element {
    let mut name = use_signal(|| profile.display_name.clone());
    let mut bio = use_signal(|| profile.bio.unwrap_or_default());
    let binding = use_form_account();
    let mut auth = use_context::<Signal<AuthContext>>();
    let mut pending = use_signal(|| false);
    let mut feedback = use_signal(|| None::<String>);
    rsx! {section{class:"card editor",h1{"Your profile"}p{class:"muted","@{profile.username}"}
      ClientForm { on_submit:move|e:FormEvent|{e.prevent_default();if pending(){return;}pending.set(true);spawn(async move{
       let initiating=binding();
    match crate::api::account::update_profile(initiating,name(),bio()).await{Ok(profile)=>{if crate::auth::account_binding::account_completion_matches(initiating,auth.read().current_user_id,profile.id){auth.write().current_user=Some(profile);feedback.set(Some("Profile saved".into()));}else{feedback.set(Some(crate::auth::account_binding::ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));}},Err(_)=>feedback.set(Some("We couldn’t save your profile. Please check your sign-in and try again.".into()))}pending.set(false);
      });},
       label{r#for:"profile-name","Display name"}input{id:"profile-name",value:name(),required:true,maxlength:100,oninput:move|e|name.set(e.value())}
       label{r#for:"profile-bio","Bio"}textarea{id:"profile-bio",rows:4,maxlength:2000,value:bio(),oninput:move|e|bio.set(e.value())}
       if let Some(message)=feedback(){p{role:"status","{message}"}}
       button{r#type:"submit",class:"btn btn-primary",disabled:pending(),if pending(){"Saving…"}else{"Save profile"}}
      }
     }}
}
#[component]
pub fn Accounts() -> Element {
    let mut accounts = use_resource(crate::api::account::linked_accounts);
    let binding = use_form_account();
    let mut pending = use_signal(|| false);
    let mut feedback = use_signal(|| None::<String>);
    let canonical =
        try_use_context::<crate::auth::local_origin::LocalCanonicalOrigin>().unwrap_or_default();
    rsx! {AuthRequired{SettingsNav{} section{class:"card editor",h1{"Sign-in methods"}p{"Link another provider while signed in. Keep at least one method linked."}
     if let Some(message)=feedback(){p{role:"status","{message}"}}
     match accounts.read().as_ref(){
      None=>rsx!{p{role:"status","Loading sign-in methods…"}},
      Some(Err(_))=>rsx!{p{role:"alert","We couldn’t load your sign-in methods."}button{class:"btn btn-secondary",onclick:move|_|accounts.restart(),"Retry"}},
      Some(Ok(items))=>rsx!{ul{class:"account-list",for account in items{li{key:"{account.id}",span{"{account.provider}"}if let Some(email)=&account.email{span{class:"muted","{email}"}}
       button{class:"btn btn-secondary",disabled:pending()||items.len()<2,onclick:{let id=account.id;move|_|{if pending(){return;}pending.set(true);spawn(async move{match crate::api::account::unlink_account(binding(),id).await{Ok(())=>{feedback.set(Some("Sign-in method removed".into()));accounts.restart();},Err(_)=>feedback.set(Some("Couldn’t remove that method. Keep at least one sign-in method linked.".into()))}pending.set(false);});}},"Unlink"}
      }}}},
     }
     div{class:"actions",a{class:"btn btn-secondary",href:canonical.on_canonical_origin("/auth/google/start?purpose=link&redirect_uri=%2Fsettings%2Faccounts"),"Link Google"}a{class:"btn btn-secondary",href:canonical.on_canonical_origin("/auth/github/start?purpose=link&redirect_uri=%2Fsettings%2Faccounts"),"Link GitHub"}}
     Link{to:Route::Profile{},"Back to profile"}
    }}}
}

#[component]
pub fn Settings() -> Element {
    let theme = use_context::<crate::utils::theme::UseTheme>();
    rsx! {
        AuthRequired {
            PageHeading { title: "Settings", description: "Make your workspace feel like yours.",
                Link { to: Route::Account {}, class: "btn btn-secondary", "View account" }
            }
            SettingsNav {}
            section { class: "quick-links", aria_label: "Account settings",
                Link { to: Route::Profile {}, class: "card quick-link", h2 { "Edit your profile" } p { "Update your display name and bio." } }
                Link { to: Route::Accounts {}, class: "card quick-link", h2 { "Manage sign-in methods" } p { "Connect a provider or review your linked accounts." } }
                Link { to: Route::Account {}, class: "card quick-link", h2 { "Review your account" } p { "View the details associated with your workspace." } }
            }
            section { class: "card page-section", id: "appearance",
                h2 { "Appearance" }
                p { class: "muted", "Choose a color scheme. This preference is saved in this browser." }
                div { class: "theme-options", role: "group", aria_label: "Color scheme",
                    button { r#type: "button", class: "btn btn-secondary theme-choice", aria_pressed: !theme.is_dark(),
                        onclick: move |_| { if theme.is_dark() { theme.toggle(); } }, "Light appearance" }
                    button { r#type: "button", class: "btn btn-secondary theme-choice", aria_pressed: theme.is_dark(),
                        onclick: move |_| { if !theme.is_dark() { theme.toggle(); } }, "Dark appearance" }
                }
            }
        }
    }
}

#[component]
pub fn Account() -> Element {
    let auth = crate::auth::context::use_auth();
    rsx! {
        AuthRequired {
            PageHeading { title: "Your account", description: "Your identity and the details of your workspace.",
                Link { to: Route::Profile {}, class: "btn btn-primary", "Edit profile" }
            }
            SettingsNav {}
            if let Some(profile) = auth.current_user {
                section { class: "card account-summary",
                    div { class: "account-monogram", aria_hidden: "true", {profile.username.chars().next().unwrap_or('U').to_uppercase().to_string()} }
                    div { class: "account-details",
                        h2 { if profile.display_name.trim().is_empty() { "{profile.username}" } else { "{profile.display_name}" } }
                        p { class: "muted", "@{profile.username}" }
                        dl { class: "detail-list",
                            div { dt { "Email" } dd { if profile.email.starts_with("noreply+") && profile.email.ends_with("@placeholder") { "Not provided" } else { "{profile.email}" } } }
                            div { dt { "Bio" } dd { if let Some(bio) = profile.bio.filter(|b| !b.trim().is_empty()) { "{bio}" } else { "You haven’t added a bio yet." } } }
                        }
                    }
                }
            } else { p { role: "status", "Loading your account…" } }
            section { class: "card page-section",
                h2 { "Sign-in and appearance" }
                p { "Review your connected providers or choose a color scheme that feels comfortable." }
                div { class: "actions",
                    Link { to: Route::Accounts {}, class: "btn btn-secondary", "Manage sign-in methods" }
                    Link { to: Route::Settings {}, class: "btn btn-secondary", "Open settings" }
                }
            }
        }
    }
}
