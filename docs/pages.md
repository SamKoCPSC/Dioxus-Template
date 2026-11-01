# Starter pages

The starter separates the public landing page from a signed-in home. These pages
are examples to adapt to each new app’s purpose and content.

| Page | Route | Access | Purpose |
| --- | --- | --- | --- |
| Landing | `/` | Public | Introduction and sign-in entry point |
| Home | `/home` | Signed in | Personal welcome, quick actions and three recent notes |
| Dashboard | `/dashboard` | Signed in | Live Notes totals, last update and six recent notes |
| Search | `/search?q=...` | Signed in | Search the current account’s note titles and content |
| Account | `/account` | Signed in | Account identity, email and bio with links to editing |
| Settings | `/settings` | Signed in | Account settings hub and light/dark appearance |
| Profile | `/settings/profile` | Signed in | Edit display name and bio |
| Sign-in methods | `/settings/accounts` | Signed in | Link/unlink providers while keeping one method |
| About | `/about` | Public | App introduction and a private-workspace explanation |
| Sign-in | `/login` | Guest | Google/GitHub OAuth entry points |
| Notes | `/notes` | Signed in | Up to 100 recent notes |
| New/edit note | `/notes/new`, `/notes/:id` | Signed in | Owner-scoped example CRUD |
| Not found | Unmatched paths | Public | Recovery navigation |

Sign-in defaults to Home and preserves a valid original return URL when supplied.
Visiting `/login` with an active session opens Home. Logout returns to the public
landing page. The primary navigation includes Home, Dashboard, Notes, Search and
Settings; About is in the footer. Settings has its own overview/account/profile/
sign-in navigation. Appearance uses the same reactive state as the header toggle
and is saved in this browser, not in a server-side account preference.

## Dashboard and Search contracts

`src/api/workspace.rs` derives the owner from the verified server session. The
client never supplies an owner ID. Dashboard counts notes and notes updated in the
last seven days; these are current-row statistics, not an edit-event history.
Refresh reads the current data again. Recent cards are bounded previews; editors
fetch the complete note separately.

Search trims a query of up to 120 Unicode characters, then performs literal,
case-insensitive substring matching against the owner’s titles and bodies. `%`,
`_`, quotes and backslashes are ordinary text. Blank queries do not list all notes.
Results are ordered by update time and ID, capped at 50, with a `has_more` flag
asking the user to refine the search. Excerpts include context around a body
match and contain at most 180 characters plus a leading ellipsis. All SQL values
are bound, all queries include the owner predicate, and responses are private and
not shared-cached.

The existing `notes(user_id, updated_at DESC, id)` index scopes these reads. This
small example needs no schema migration, external search service or PostgreSQL
extension. Larger products should add appropriate pagination, quotas and a search
index based on measured query plans and their domain’s language requirements.

The search form uses native GET navigation so the query survives reloads and
history navigation. Search and sign-in read the raw history URL and parse its
fields with `url::form_urlencoded`: Dioxus 0.7.9’s typed-query parser decodes the
whole query before splitting fields, which loses literal ampersands and nested
return URLs. Keep the special-character and browser-history regressions when
upgrading that routing code.

## Extension points and verification

- `src/main.rs`: route inventory.
- `src/components/mod.rs`: navigation and shared theme context.
- `src/components/workspace.rs`: page headings, preview cards and settings tabs.
- `src/pages/mod.rs`: public landing, About, sign-in and not-found pages.
- `src/pages/workspace.rs`: Home, Dashboard, Search and raw search-query decoding.
- `src/pages/settings.rs`: Settings, Account, profile and provider screens.
- `src/api/workspace.rs`: Notes-based summary/search read models.
- `src/utils/query.rs`: once-only query decoding for Search and sign-in return URLs.
- `src/middleware/auth.rs`: private page gates and safe return URLs.

Read pages remount with the authenticated identity through SessionPage; old
account results must not appear after a session change. Editors retain their
existing initiating-account protection. Adding a private page requires both the
route gate and server-side authorization for its data calls.

Run `just check`, `just lint`, `just test`, and `just config-test`. The browser
smoke script exercises public pages, guest gates, saved-note search with literal
special characters, query reload/back navigation, a second isolated synthetic
account, live Dashboard updates, settings/profile behavior, mobile layouts and
logout. Inspect the desktop/mobile captures outside the source checkout.
