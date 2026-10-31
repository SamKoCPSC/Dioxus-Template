# Optional object storage

The default application builds without the AWS SDK and needs no hosted storage
credentials. Local development starts a MinIO fixture for apps that add media,
but STORAGE_ENABLED remains false until that feature is configured.

The server adapter in src/storage.rs supports S3-compatible endpoints (including
R2 and MinIO). It is compiled only with `storage-s3`, is excluded from WASM, and
uses explicit endpoint/region/bucket credentials. It does not inspect the host's
unrelated cloud identities.

To include it in a bundled app, use:

```bash
dx bundle --platform web --release --force-sequential true --features storage-s3
```

For a production Docker build set `--build-arg APP_FEATURES=storage-s3`. Then set
STORAGE_ENABLED=true and provide the S3 variables in .env.production.example via
the hosting secret manager. Builds without the feature reject STORAGE_ENABLED=true
with an actionable startup message. S3_REGION defaults to `auto` for R2; set the
actual region for other providers. Public endpoint URLs must use HTTPS on hosting.

`ObjectStore::upload_public_webp` accepts session-derived owner IDs and bounded
WebP bytes, generates a UUID object key, and returns a typed StoredObject.
`delete_owned` accepts that typed reference and rejects another owner. These are
building blocks, not a finished attachment/avatar feature. Do not expose them
unchanged as a client-controlled URL/key deletion endpoint.

Before adding an upload UI/API:

1. Define whether the content is public or private. This adapter returns public
   media URLs; private notes/documents require a separate private delivery design.
2. Track attachments and references in the database and bind every write to the
   authenticated owner and initiating form account.
3. Validate content and request sizes, enforce quotas/concurrency bounds, and
   implement rollback/orphan cleanup when storage or database writes fail.
4. Decide retention/reference ownership independently of the uploading user when
   content can be shared, versioned, or transferred.
5. Add integration tests for owner isolation, failed commits, replacement/deletion
   and retry behavior against an isolated bucket/MinIO fixture.

Never reuse a live Noms bucket, endpoint credentials, account identity or object
namespace for a new app. New apps/environments need independent resources.
