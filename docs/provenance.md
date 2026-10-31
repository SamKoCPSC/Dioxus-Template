# Extraction provenance and scope

Source: https://github.com/SamKoCPSC/Noms
Source commit: `5f94d121658ec7820e1654e5625cbe793a71bf50`
Extraction date: 2026-10-31
Destination: https://github.com/SamKoCPSC/Dioxus-Template

The repository owner requested this extraction. Source authentication/session,
proxy trust, redirect, local-origin, UI interaction/lifecycle and isolated local
infrastructure code was adapted into a separate application. Generic behavior
regressions were retained. Tests asserting removed recipe/collection/graph screens
were removed or adapted to the Notes route; they are not claims of coverage for
those omitted products.

The database identity/session operations were separated from Noms' mixed database
module. Queries use runtime SQLx bindings and a small extension-free PostgreSQL 17
schema. Periodic auth/session cleanup moved to the server task. Product-specific
media/reference policy, search/vocabulary schema/seeding, recipes, pantry/grocery,
importers, artwork/fonts and deployment-locked staging login were not copied.

New work includes a neutral UI shell, private Notes example, profile/account
screens, app identity configuration, optional generic storage adapter, agent/user
setup runbooks and template-focused validation. Shared-crate synchronization and
native app distribution are deliberately outside this first extraction.

Third-party packages retain their own licenses. No Noms application data, live
credentials, browser state, database/media volumes or production service binding
belongs in the template. The maintained application package name is `app` so a
new product does not require renaming executable/bundle/import paths.
