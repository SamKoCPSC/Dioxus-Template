use crate::components::ClientForm;
use crate::{
    auth::account_binding::use_form_account,
    components::{base::SharedModal, AuthRequired},
    types::Note,
    Route,
};
use dioxus::prelude::*;
use uuid::Uuid;
#[component]
pub fn Notes() -> Element {
    let mut notes = use_resource(crate::api::notes::list_notes);
    rsx! {
     AuthRequired {
      section {
       div { class:"page-heading",
        div { p {class:"eyebrow","Your workspace"} h1 {"My notes"} }
        Link {to:Route::NewNote{},class:"btn btn-primary","New note"}
       }
       match notes.read().as_ref() {
        None=>rsx!{p{role:"status","Loading notes…"}},
        Some(Err(_))=>rsx!{div{class:"card",p{role:"alert","We couldn’t load your notes."}button{class:"btn btn-secondary",onclick:move|_|notes.restart(),"Retry"}}},
        Some(Ok(items)) if items.is_empty()=>rsx!{div{class:"card empty-state",h2{"A little space for your ideas"}p{"Create your first note to get started."}}},
        Some(Ok(items))=>rsx!{
         div {class:"notes-grid",
          for note in items {
           Link {key:"{note.id}",to:Route::EditNote{id:note.id},class:"card note-card",
            h2 {"{note.title}"}
            p { {note.body.chars().take(180).collect::<String>()} }
            small { {format!("Updated {}",note.updated_at.format("%Y-%m-%d %H:%M UTC"))} }
           }
          }
         }
         p {class:"muted","Showing up to 100 most recently updated notes."}
        },
       }
      }
     }
    }
}
#[component]
pub fn NewNote() -> Element {
    rsx! {AuthRequired{NoteForm{initial:None}}}
}
#[component]
pub fn EditNote(id: Uuid) -> Element {
    let mut note = use_resource(move || crate::api::notes::get_note(id));
    rsx! {AuthRequired{match note.read().as_ref(){
     None=>rsx!{p{role:"status","Loading note…"}},
     Some(Err(_))=>rsx!{section{class:"card",h1{"Note unavailable"}p{"It may have been deleted or belong to another account."}button{class:"btn btn-secondary",onclick:move|_|note.restart(),"Retry"}Link{to:Route::Notes{},"Back to notes"}}},
     Some(Ok(note))=>rsx!{NoteForm{key:"{note.id}",initial:Some(note.clone())}},
    }}}
}
#[component]
fn NoteForm(initial: Option<Note>) -> Element {
    let id = initial.as_ref().map(|n| n.id);
    let mut title = use_signal(|| {
        initial
            .as_ref()
            .map(|n| n.title.clone())
            .unwrap_or_default()
    });
    let mut body = use_signal(|| initial.as_ref().map(|n| n.body.clone()).unwrap_or_default());
    let mut pending = use_signal(|| false);
    let mut feedback = use_signal(|| None::<String>);
    let mut confirming = use_signal(|| false);
    let current_auth = use_context::<Signal<crate::auth::context::AuthContext>>();
    let binding = use_form_account();
    let nav = use_navigator();
    rsx! {section{class:"editor card",h1{if id.is_some(){"Edit note"}else{"New note"}}
      ClientForm { on_submit:move|event:FormEvent|{event.prevent_default();if pending(){return;}pending.set(true);feedback.set(None);spawn(async move{
       let initiating=binding();
    let result=crate::api::notes::save_note(initiating,id,title(),body()).await;
       pending.set(false);match result{Ok(saved)=>{if initiating.is_some_and(|owner|crate::auth::account_binding::account_completion_matches(initiating,current_auth.read().current_user_id,owner)){feedback.set(Some("Saved".into()));nav.push(Route::EditNote{id:saved.id});}else{feedback.set(Some(crate::auth::account_binding::ACCOUNT_COMPLETION_CHANGED_FEEDBACK.into()));}},Err(error)=>feedback.set(Some(crate::auth::account_binding::account_error_feedback(&error).unwrap_or("We couldn’t save this note. Your entries are still here; check your sign-in and try again.").into()))}
      });},
       label{r#for:"note-title","Title"}input{id:"note-title",required:true,maxlength:160,value:title(),oninput:move|e|title.set(e.value())}
       label{r#for:"note-body","Note"}textarea{id:"note-body",rows:12,maxlength:20000,value:body(),oninput:move|e|body.set(e.value())}
       if let Some(message)=feedback(){p{role:"status","{message}"}}
       div{class:"actions",button{r#type:"submit",class:"btn btn-primary",disabled:pending(),if pending(){"Saving…"}else{"Save note"}}Link{to:Route::Notes{},class:"btn btn-secondary","Back to notes"}
        if id.is_some(){button{r#type:"button",class:"btn btn-danger",disabled:pending(),onclick:move|_|confirming.set(true),"Delete note"}}
       }
      }
      if confirming(){SharedModal{aria_labelledby:"delete-title",focus_selector:"#cancel-delete",pending:pending(),on_dismiss:move|_|confirming.set(false),
       div{class:"modal-card",h2{id:"delete-title","Delete this note?"}p{"This permanently removes the note from your account."}div{class:"actions",
        button{id:"cancel-delete",class:"btn btn-secondary",disabled:pending(),onclick:move|_|confirming.set(false),"Keep note"}
        button{class:"btn btn-danger",disabled:pending(),onclick:move|_|{if pending(){return;}pending.set(true);spawn(async move{
         let result=crate::api::notes::delete_note(binding(),id.expect("existing note")).await;
         pending.set(false);match result{Ok(())=>{nav.push(Route::Notes{});},Err(_)=>{confirming.set(false);feedback.set(Some("We couldn’t delete this note. Please try again.".into()));}}
        });},"Delete permanently"}
       }}
      }}
     }}
}
