//! Optional, server-only S3/R2/MinIO adapter. New features own database attachment references.
#![allow(dead_code)]
use aws_sdk_s3::{
    config::{Credentials, Region},
    primitives::ByteStream,
    Client,
};
use uuid::Uuid;
#[derive(Clone)]
pub struct ObjectStore {
    client: Client,
    bucket: String,
    public_url: url::Url,
}
impl ObjectStore {
    pub async fn from_environment() -> Result<Option<Self>, String> {
        match std::env::var("STORAGE_ENABLED").as_deref() {
            Err(_) | Ok("") | Ok("false") => return Ok(None),
            Ok("true") => {}
            _ => return Err("STORAGE_ENABLED must be true or false".into()),
        }
        let required = |key: &str| std::env::var(key).map_err(|_| format!("{key} is required"));
        let endpoint = required("S3_ENDPOINT")?;
        let public_url =
            url::Url::parse(&required("S3_PUBLIC_URL")?).map_err(|_| "Invalid S3_PUBLIC_URL")?;
        let access = required("S3_ACCESS_KEY_ID")?;
        let secret = required("S3_SECRET_ACCESS_KEY")?;
        let config = aws_sdk_s3::config::Builder::new()
            .behavior_version_latest()
            .region(Region::new(
                std::env::var("S3_REGION").unwrap_or_else(|_| "auto".into()),
            ))
            .endpoint_url(endpoint)
            .force_path_style(true)
            .credentials_provider(Credentials::new(
                access,
                secret,
                None,
                None,
                "app-configuration",
            ))
            .build();
        Ok(Some(Self {
            client: Client::from_conf(config),
            bucket: required("S3_BUCKET")?,
            public_url,
        }))
    }
    /// Authority must be a session-derived owner, never an arbitrary client path.
    /// These objects are public media. Use signed URLs/private buckets for private files.
    pub async fn upload_public_webp(
        &self,
        owner: Uuid,
        bytes: Vec<u8>,
    ) -> Result<StoredObject, String> {
        if bytes.len() < 12
            || bytes.len() > 2 * 1024 * 1024
            || &bytes[..4] != b"RIFF"
            || &bytes[8..12] != b"WEBP"
        {
            return Err("Expected a WebP image of at most 2 MiB".into());
        }
        let id = Uuid::new_v4();
        let key = object_key(owner, id);
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .content_type("image/webp")
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|_| "Storage upload failed")?;
        let url = format!("{}/{}", self.public_url.as_str().trim_end_matches('/'), key);
        Ok(StoredObject { owner, id, url })
    }
    /// Caller must prove reference ownership before deletion; only typed generated keys are used.
    pub async fn delete_owned(&self, owner: Uuid, object: &StoredObject) -> Result<(), String> {
        if object.owner != owner {
            return Err("Storage ownership mismatch".into());
        }
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(object_key(owner, object.id))
            .send()
            .await
            .map_err(|_| "Storage deletion failed")?;
        Ok(())
    }
}
pub struct StoredObject {
    owner: Uuid,
    id: Uuid,
    pub url: String,
}
fn object_key(owner: Uuid, id: Uuid) -> String {
    format!("media/{owner}/{id}.webp")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn object_keys_are_owner_scoped() {
        let owner = Uuid::from_u128(1);
        let id = Uuid::from_u128(2);
        assert_eq!(object_key(owner, id), format!("media/{owner}/{id}.webp"));
        assert_ne!(object_key(owner, id), object_key(Uuid::from_u128(3), id));
    }
}
