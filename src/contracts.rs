use alloy::sol;
use std::hash::Hash;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, Hash)]
    Bridge,
    "contract/combined/Bridge.json"
);
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, Hash)]
    console,
    "contract/combined/console.json"
);
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, Hash)]
    DemoERC20,
    "contract/combined/DemoERC20.json",
);
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, Hash)]
    TestERC20,
    "contract/combined/TestERC20.json",
);
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, Hash)]
    ExmERC20,
    "contract/combined/ExmERC20.json",
);

#[macro_export]
macro_rules! provider {
    ($key: ident) => {{
        use alloy::providers::ProviderBuilder;
        use $crate::RPC_URL;

        ProviderBuilder::new()
            .wallet($key.clone())
            .connect(RPC_URL)
            .await
            .unwrap()
    }};
}
#[macro_export]
macro_rules! contract_provider {
    ($contract_type: tt, $key: ident) => {
        {
            use $crate::RPC_URL;
            use alloy::{
                primitives::Address,
                providers::ProviderBuilder,
            };

            use eyre::Context;
            use std::{
                fs,
                hash::{DefaultHasher, Hash, Hasher},
                path::PathBuf,
                str::FromStr,
            };

            let provider = ProviderBuilder::new().wallet($key.clone()).connect(RPC_URL).await.unwrap();

            let mut hasher = DefaultHasher::new();
            $contract_type::BYTECODE.hash(&mut hasher);
            let hash = hasher.finish();

            let path = PathBuf::from_str(&format!("/tmp/{hash}.address")).context("Невалидное имя для файла").unwrap();

            if path.exists() {
                let contract_address_str =
                    fs::read_to_string(&path).context("При чтении адресса контракта произошла ошибка").unwrap();
                let contract_address = contract_address_str
                    .trim_end()
                    .parse::<Address>()
                    .context("Неудалось преобразовать строку в адрес").unwrap();
                $contract_type::new(contract_address, provider.clone())
            }else{
                let contract = $contract_type::deploy(provider).await.unwrap();
                let contract_address = contract.address();
                println!("Deployed contract at address: {contract_address}");
                fs::write(&path, contract_address.to_string())
                    .context("При записи адреса контракта произошла ошибка").unwrap();

                contract
            }
        }
    };
}

#[macro_export]
macro_rules! token_fund {
    ($token_contract: ident, $users: ident) => {{
        use std::str::FromStr;

        println!("");

        let name = $token_contract.name().call().await.unwrap().to_string();
        println!("Name: {name}");

        let sybmol = $token_contract.symbol().call().await.unwrap().to_string();
        println!("Symbol: {sybmol}");

        let decimals = $token_contract.decimals().call().await.unwrap().to_string();
        println!("Decimals: {decimals}");

        let min = U256::from(10).pow(U256::from_str(&decimals).unwrap()) * U256::from(10);

        for addr in $users.clone() {
            let old_balance = $token_contract.balanceOf(addr).call().await.unwrap();
            if old_balance > min {
                println!("Баланс в норме {addr}");
                continue;
            }

            let tx = $token_contract
                .transfer(addr, min)
                .send()
                .await
                .unwrap()
                .watch()
                .await
                .unwrap();
            println!("Transfer to {addr} {min}:\n{tx:?}");
        }
    }};
}

#[macro_export]
macro_rules! tokens_balans {
    ($user_address: ident, $( $tokens:expr ),*) => {{
        let mut temp_vec = Vec::new();
        $(
            let balance = $tokens
                .balanceOf($user_address)
                .call()
                .await
                .unwrap();
            temp_vec.push(balance);
        )*
        temp_vec
    }};
}
