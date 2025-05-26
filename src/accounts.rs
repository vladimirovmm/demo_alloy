use std::{fs, path::PathBuf, str::FromStr};

use alloy::{
    primitives::FixedBytes,
    signers::{k256::ecdsa::SigningKey, local::LocalSigner},
};
use eyre::{Context, Result};
use tracing::debug;

pub type Signer = LocalSigner<SigningKey>;

pub(crate) async fn read_accounts() -> Result<Vec<Signer>> {
    let key_path = PathBuf::from_str("keys.private").context("Недопустимое значение пути")?;

    if !key_path.exists() {
        // Чтение из директории geth keystore
        let keys = read_keystore_from_geth().await?;

        // Сохранение ключей в файл в расшифрованном виде
        let keys_bytes: Vec<_> = keys.iter().map(|v| v.to_bytes()).collect();
        let keys_json = serde_json::to_string_pretty(&keys_bytes)
            .context("Неудалось преобразовать ключ в JSON")?;
        fs::write(&key_path, keys_json).context("Ошибка при записи файла ключей")?;
        return Ok(keys);
    }

    let keys_json = fs::read_to_string(&key_path).context("Неудалось прочитать файл ключей")?;
    let keys_bytes = serde_json::from_str::<Vec<FixedBytes<32>>>(&keys_json)
        .context("Неудалось преобразовать JSON в ключи")?;
    keys_bytes
        .into_iter()
        .map(|v| LocalSigner::from_bytes(&v).context("Ошибка при преобразовании байтов в ключ"))
        .collect::<Result<Vec<_>>>()
}

/// Чтение ключей из директории geth (keystore)
async fn read_keystore_from_geth() -> Result<Vec<Signer>> {
    let keystore_path = PathBuf::from_str("data/keystore").context("Путь до ключей не валиден")?;

    let read_keys = keystore_path
        .read_dir()
        .context("Неудалось прочитать директорию")?
        .filter_map(|v| v.ok())
        .map(|v| v.path())
        .filter(|v| v.is_file())
        .inspect(|v| debug!("Ключ: {v:?}"))
        .map(|path| async move { LocalSigner::decrypt_keystore(&path, "") });
    futures_util::future::try_join_all(read_keys)
        .await
        .context("Ошибка при декодировании ключей")
}
