use crate::components::ClientForm;
use crate::{
    auth::{account_binding::use_form_account, context::AuthContext},
    components::AuthRequired,
    Route,
};
use dioxus::prelude::*;
#[component]
pub fn Profile() -> Element {
    let auth = crate::auth::context::use_auth();
    rsx! {AuthRequired{if let Some(profile)=auth.current_user{ProfileForm{profile}}}}
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
    rsx! {AuthRequired{section{class:"card editor",h1{"Sign-in methods"}p{"Link another provider while signed in. Keep at least one method linked."}
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
