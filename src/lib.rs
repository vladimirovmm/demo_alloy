#[cfg(test)]
mod tests {
    use alloy::{
        primitives::{Address, FixedBytes, U256, Uint},
        providers::{Provider, ProviderBuilder, fillers::FillProvider},
        rpc::types::Filter,
        signers::{k256::ecdsa::SigningKey, local::LocalSigner},
        sol_types::sol,
    };

    use Bridge::BridgeInstance;
    use DemoERC20::DemoERC20Instance;
    use alloy_sol_types::SolEvent;
    use console::consoleInstance;
    use eyre::{Context, Result};
    use std::{
        fs,
        hash::{DefaultHasher, Hash, Hasher},
        path::PathBuf,
        str::FromStr,
    };
    use tokio::sync::mpsc;
    use tracing_test::traced_test;

    type Signer = LocalSigner<SigningKey>;

    sol!(
        #[allow(missing_docs)]
        #[sol(rpc)]
        #[derive(Debug, Hash)]
        Bridge,
        "contract/Bridge.json"
    );
    sol!(
        #[allow(missing_docs)]
        #[sol(rpc)]
        #[derive(Debug, Hash)]
        console,
        "contract/console.json"
    );

    sol! (
        #[allow(missing_docs)]
        #[derive(Debug, PartialEq, Eq)]
        library Errors {
            error InsFund(uint, uint, address);
        }
    );

    sol!(
        #[allow(missing_docs)]
        #[sol(rpc)]
        #[derive(Debug, Hash)]
        DemoERC20,
        "contract/DemoERC20.json",
    );

    const RPC_URL: &str = "http://localhost:8545";

    fn read_accounts() -> Result<Vec<Signer>> {
        fn read_keystore_from_geth() -> Result<Vec<Signer>> {
            let keys_path =
                PathBuf::from_str("data/keystore").context("Путь до ключей не валиден")?;
            let keys = keys_path
                .read_dir()
                .context("Неудалось прочитать директорию")?
                .filter_map(|v| v.ok())
                .map(|v| v.path())
                .filter(|v| v.is_file())
                .inspect(|v| {
                    dbg!(&v);
                })
                .map(|v| LocalSigner::decrypt_keystore(&v, ""))
                .filter_map(|v| v.ok())
                .collect::<Vec<_>>();

            Ok(keys)
        }

        let key_path = PathBuf::from_str("keys.private").unwrap();
        if !key_path.exists() {
            let keys = read_keystore_from_geth()?;

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

    async fn init<T, A>(
        provider: FillProvider<T, A>,
    ) -> Result<(
        BridgeInstance<FillProvider<T, A>>,
        DemoERC20Instance<FillProvider<T, A>>,
        consoleInstance<FillProvider<T, A>>,
    )>
    where
        T: alloy::providers::fillers::TxFiller,
        A: alloy::providers::Provider + Clone,
    {
        // check
        let hashs: [u64; 3] =
            [&Bridge::BYTECODE, &DemoERC20::BYTECODE, &console::BYTECODE].map(|byte_code| {
                let mut hasher = DefaultHasher::new();
                byte_code.hash(&mut hasher);
                hasher.finish()
            });

        let paths: Vec<PathBuf> = hashs
            .iter()
            .map(|hash| {
                PathBuf::from_str(&format!("/tmp/{hash}.address"))
                    .context("Невалидное имя для файла")
            })
            .collect::<Result<_>>()
            .unwrap();

        if paths.iter().all(|path| path.exists()) {
            let addresses = paths
                .iter()
                .map(|path| {
                    let address_str = fs::read_to_string(path)
                        .context("При чтении адресса контракта произошла ошибка")?;
                    address_str
                        .trim_end()
                        .parse::<Address>()
                        .context("Неудалось преобразовать строку в адрес")
                })
                .collect::<Result<Vec<_>>>()
                .unwrap();

            return Ok((
                Bridge::new(addresses[0], provider.clone()),
                DemoERC20::new(addresses[1], provider.clone()),
                consoleInstance::new(addresses[2], provider.clone()),
            ));
        }

        let bridge_contract_client = Bridge::deploy(provider.clone()).await?;
        let bridge_contract_address = bridge_contract_client.address();
        println!("Deployed contract at address: {bridge_contract_address}",);
        fs::write(&paths[0], bridge_contract_address.to_string())
            .context("При записи адреса контракта произошла ошибка")?;

        let demo_erc20_contract_client = DemoERC20::deploy(provider.clone()).await?;
        let demo_erc20_contract_address = demo_erc20_contract_client.address();
        println!("Deployed contract at address: {demo_erc20_contract_address}",);
        fs::write(&paths[1], demo_erc20_contract_address.to_string())
            .context("При записи адреса контракта произошла ошибка")?;

        let console_contract_client = console::deploy(provider.clone()).await?;
        let console_contract_address = console_contract_client.address();
        println!("Deployed contract at address: {console_contract_address}",);
        fs::write(&paths[2], console_contract_address.to_string())
            .context("При записи адреса контракта произошла ошибка")?;

        Ok((
            bridge_contract_client,
            demo_erc20_contract_client,
            console_contract_client,
        ))
    }

    #[traced_test]
    #[tokio::test]
    async fn test_depoloy_contracts() {
        let accounts = read_accounts().unwrap();
        let owner = accounts[0].clone();

        let owner_provider = ProviderBuilder::new()
            .wallet(owner.clone())
            .connect(RPC_URL)
            .await
            .unwrap();

        init(owner_provider.clone()).await.unwrap();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_bridge() {
        let accounts = read_accounts().unwrap();
        let owner = accounts[0].clone();
        let alice = accounts[1].clone();
        let bob = accounts[2].clone();

        println!("owner: {}", owner.address());
        println!("alice: {}", alice.address());
        println!("bob: {}", bob.address());

        // Клиент для owner
        let owner_provider = ProviderBuilder::new()
            .wallet(owner.clone())
            .connect(RPC_URL)
            .await
            .unwrap();
        // Клиент для alice
        let alice_provider = ProviderBuilder::new()
            .wallet(alice.clone())
            .connect(RPC_URL)
            .await
            .unwrap();
        // Клиент для bob
        let bob_provider = ProviderBuilder::new()
            .wallet(bob.clone())
            .connect(RPC_URL)
            .await
            .unwrap();
        // Клиент контракта для owner
        let (bridge_owner, demo_token_owner, ..) = init(owner_provider.clone()).await.unwrap();
        // Клиент контракта для alice
        let (bridge_alice, demo_token_alice, ..) = init(alice_provider.clone()).await.unwrap();
        // Клиент контракта для bob
        let (bridge_bob, demo_token_bob, ..) = init(bob_provider.clone()).await.unwrap();

        assert_eq!(bridge_owner.address(), bridge_alice.address());
        assert_eq!(bridge_bob.address(), bridge_alice.address());
        assert_eq!(demo_token_owner.address(), demo_token_alice.address());
        println!("Bridge address: {}", bridge_owner.address());
        println!("DemoERC20 address: {}", demo_token_owner.address());

        let filter = Filter::new();
        let watch_send_event = owner_provider.watch_logs(&filter).await.unwrap();
        let _console_handle = tokio::spawn(async move {
            use futures_util::StreamExt;

            let mut k = watch_send_event.into_stream();
            while let Some(logs) = k.next().await {
                for log in logs {
                    let Some(log_topic) = log.topic0() else {
                        continue;
                    };

                    match *log_topic {
                        DemoERC20::Approval::SIGNATURE_HASH => {
                            let msg = log.log_decode::<DemoERC20::Approval>().unwrap();
                            println!("Log: {:#?}", msg.data());
                        }
                        console::VoteString::SIGNATURE_HASH => {
                            let msg = log.log_decode::<console::VoteString>().unwrap();
                            println!("Log: {:#?}", msg.data());
                        }
                        console::VoteAdderss::SIGNATURE_HASH => {
                            let msg = log.log_decode::<console::VoteAdderss>().unwrap();
                            println!("Log: {:#?}", msg.data());
                        }
                        console::VoteNumber::SIGNATURE_HASH => {
                            let msg = log.log_decode::<console::VoteNumber>().unwrap();
                            println!("Log: {:#?}", msg.data());
                        }
                        _ => (),
                    }
                }
            }
        });

        println!("# # # Перевод ETH # # #");

        println!("Создание отслеживание события переводов на L2 ETH");
        let watch_send_event = bridge_owner.Send_filter().watch().await.unwrap();
        let (s, mut r) = mpsc::channel(10);
        let event_handle = tokio::spawn(async move {
            use futures_util::StreamExt;

            let mut k = watch_send_event.into_stream();
            while let Some(Ok((event, _l))) = k.next().await {
                s.send(event).await.unwrap();
            }
        });

        println!("переводов на L2 ETH");
        let amount = U256::from(1_000_000_000);
        for (name, address, contract) in [
            ("alice", alice.address(), bridge_alice.clone()),
            ("bob", bob.address(), bridge_bob.clone()),
        ] {
            println!();
            print!("Перевод для {name} {amount} ETH: ");
            // Старый баланс ETH
            let old_balance = owner_provider.get_balance(address).await.unwrap();
            let old_balance_owner = owner_provider.get_balance(owner.address()).await.unwrap();
            // Транзакция перевода на L2
            let tx = contract
                .deposite(address)
                .value(amount)
                .send()
                .await
                .unwrap()
                .watch()
                .await
                .unwrap();
            println!("{tx} tx");
            // Новый баланс ETH
            let new_balance = owner_provider.get_balance(address).await.unwrap();
            println!(
                "{name}:\n\
                old: {old_balance}\n\
                new: {new_balance}"
            );
            assert!(new_balance < old_balance - amount); // Сумма перевода и комиссия

            // owner баланс
            let new_balance_owner = owner_provider.get_balance(owner.address()).await.unwrap();
            println!(
                "owner: \n\
                old: {old_balance_owner}\n\
                new: {new_balance_owner}"
            );
            assert_eq!(new_balance_owner, old_balance_owner + amount);
        }

        bridge_bob
            .deposite(alice.address())
            .value(Uint::from(9))
            .send()
            .await
            .unwrap()
            .watch()
            .await
            .unwrap();

        println!();
        println!("Тестирования мониторинга событий");
        let t1 = r.recv().await.unwrap();
        println!("Tx[1]: {t1:#?}");
        assert_eq!(t1.from, alice.address());
        assert_eq!(t1.to, alice.address());
        assert_eq!(t1.value, amount);

        let t2 = r.recv().await.unwrap();
        println!("Tx[2]: {t2:#?}");
        assert_eq!(t2.from, bob.address());
        assert_eq!(t2.to, bob.address());
        assert_eq!(t2.value, amount);

        let t3 = r.recv().await.unwrap();
        println!("Tx[3]: {t3:#?}");
        assert_eq!(t3.from, bob.address());
        assert_eq!(t3.to, alice.address());
        assert_eq!(t3.value, Uint::from(9));

        event_handle.abort();

        println!("Тестирование перевода/withdrow на L1 ETH");

        let old_balance = owner_provider.get_balance(owner.address()).await.unwrap();
        print!("Резервирование средств для вывода: ");
        let tx = bridge_owner
            .request_withdraw(alice.address())
            .value(Uint::from(1_000_000_000_000_000_u64))
            .send()
            .await
            .unwrap()
            .watch()
            .await
            .unwrap();
        println!("{tx} tx");
        let new_balance_owner = owner_provider.get_balance(owner.address()).await.unwrap();
        assert!(new_balance_owner < old_balance - amount); // Сумма перевода и комиссия
        println!(
            "Owner баланс: \n\
            old: {old_balance}\n\
            new {new_balance_owner}"
        );

        let withdrow_balance = bridge_alice.status_withdraw().call().await.unwrap();
        println!("Готово на вывод: {withdrow_balance}");

        let old_balance = alice_provider.get_balance(alice.address()).await.unwrap();
        print!("Alice завершает вывод");
        let tx = bridge_alice
            .withdraw()
            .send()
            .await
            .unwrap()
            .watch()
            .await
            .unwrap();
        println!("{tx} tx");
        let new_balance = alice_provider.get_balance(alice.address()).await.unwrap();
        println!(
            "Alice баланс: \n\
            old: {old_balance} eth\n\
            new {new_balance} eth"
        );
        assert!(new_balance > old_balance);

        // Проверка баланса owner на то что он не изменился и снятие произошло при withdraw request
        let balance_owner = owner_provider.get_balance(owner.address()).await.unwrap();
        assert_eq!(balance_owner, new_balance_owner);

        let withdrow_balance = bridge_alice.status_withdraw().call().await.unwrap();
        assert_eq!(withdrow_balance, Uint::ZERO);

        println!("# # # Перевод ERC20 токена # # #");
        println!("Тестирование переводов на L2 ERC20");

        // # # # тестирование пополнения ERC20 токена # # #

        println!("Проверка баланса owner ERC20");
        let balance = demo_token_owner
            .balanceOf(owner.address())
            .call()
            .await
            .unwrap();
        assert!(balance > Uint::from(2_000_000_000_000_u64));

        println!("Проверка баланса ERC20 для BOB и Alice. Хватит ли на тесты");
        for address in [alice.address(), bob.address()] {
            let balance = demo_token_owner.balanceOf(address).call().await.unwrap();
            println!("DemoERC20 {address}: {balance}");
            if balance > Uint::from(1_000_000_000) {
                continue;
            }
            demo_token_owner
                .transfer(address, Uint::from(1_000_000_000_000_u64)) // Такой перевод возможен только от владельца токена
                .send()
                .await
                .unwrap()
                .watch()
                .await
                .unwrap();
        }

        let amount_deposit = Uint::from(1_000_000);

        for (name, key, bridge, token) in [
            (
                "alice",
                alice.clone(),
                bridge_alice.clone(),
                demo_token_alice.clone(),
            ),
            (
                "bob",
                bob.clone(),
                bridge_bob.clone(),
                demo_token_bob.clone(),
            ),
        ] {
            let old_balance_owner = token.balanceOf(owner.address()).call().await.unwrap();
            let old_balance = token.balanceOf(key.address()).call().await.unwrap();
            println!("DemoERC20 Owner: {old_balance_owner}");
            println!("DemoERC20 {name}: {old_balance}");

            println!("Перевод {name} {amount_deposit} ERC20 токенов на L2");

            // Передаём возможность контракту забрать токены в указаном количестве
            let tx = token
                .approve(*bridge.address(), amount_deposit)
                .send()
                .await
                .unwrap()
                .watch()
                .await
                .unwrap();
            println!("{name} одобрил перевод на Адрес контракта {tx}"); // Только контракт сможет забрать эти токины

            // Проверяем может ли контакт забрать эти токены
            let allow_amount = token
                .allowance(key.address(), *bridge.address())
                .call()
                .await
                .unwrap();

            assert_eq!(allow_amount, amount_deposit);

            // Переводим на L2
            let tx = bridge
                .deposite_erc20(*token.address(), key.address(), amount_deposit)
                .send()
                .await
                .unwrap()
                .watch()
                .await
                .unwrap();
            println!("[{name}] tx deposite_erc20: {tx}");

            println!("Проверка OWNER баланса на L2 ERC20");
            let new_balance_owner = token.balanceOf(owner.address()).call().await.unwrap();
            println!("DemoERC20 owner: {new_balance_owner}");
            assert_eq!(
                new_balance_owner,
                old_balance_owner + amount_deposit,
                "Баланс пользователя не изменился\n\
                {old_balance_owner}:{new_balance_owner}"
            );

            let new_balance = token.balanceOf(key.address()).call().await.unwrap();
            println!("DemoERC20 {name}: {new_balance}");
            assert_eq!(new_balance, old_balance - amount_deposit,)
        }

        // withdraw ERC20
        // Запрос может сделать только owner
        let amount = Uint::from(10);
        println!("Запрос на вывод {amount} ERC20 токенов от owner на");

        for (name, key, bridge, token) in [
            (
                "alice",
                alice.clone(),
                bridge_alice.clone(),
                demo_token_alice.clone(),
            ),
            (
                "bob",
                bob.clone(),
                bridge_bob.clone(),
                demo_token_bob.clone(),
            ),
        ] {
            let token_address = *token.address();
            let bridge_address = *bridge.address();

            let old_balance_owner = token.balanceOf(owner.address()).call().await.unwrap();
            let old_balance = token.balanceOf(key.address()).call().await.unwrap();

            println!("Резервирование средств ERC20 на первод owner => {name}");
            let tx = demo_token_owner
                .approve(bridge_address, amount)
                .send()
                .await
                .unwrap()
                .watch()
                .await
                .unwrap();
            println!("approve: {tx}");

            let allow = token
                .allowance(owner.address(), bridge_address)
                .call()
                .await
                .unwrap();
            assert!(allow >= amount);

            println!("Запрос на вывод {amount} ERC20 токенов от owner на {name}");
            let tx = bridge_owner
                .request_withdraw_erc20(token_address, key.address(), amount)
                .send()
                .await
                .unwrap()
                .watch()
                .await
                .unwrap();
            println!("request withdraw tx: {tx}");

            let witrhdraw_amount = bridge
                .status_withdraw_erc20(token_address)
                .call()
                .await
                .unwrap();
            assert!(witrhdraw_amount >= amount);

            println!("Завершение перевода {amount} ERC20 токенов на {name}");
            let tx = bridge
                .withdraw_erc20(token_address)
                .send()
                .await
                .unwrap()
                .watch()
                .await
                .unwrap();
            println!("withdraw tx: {tx}");

            let new_balance_owner = token.balanceOf(owner.address()).call().await.unwrap();
            assert_eq!(new_balance_owner, old_balance_owner - amount);

            let new_balance = token.balanceOf(key.address()).call().await.unwrap();
            assert_eq!(new_balance, old_balance + amount);

            let witrhdraw_amount = bridge
                .status_withdraw_erc20(token_address)
                .call()
                .await
                .unwrap();
            assert_eq!(witrhdraw_amount, Uint::ZERO);
        }
    }

    #[traced_test]
    #[tokio::test]
    async fn test_token() {
        let accounts = read_accounts().unwrap();
        let owner = accounts[0].clone();
        let alice = accounts[1].clone();

        let owner_provider = ProviderBuilder::new()
            .wallet(owner.clone())
            .connect(RPC_URL)
            .await
            .unwrap();
        let alice_provider = ProviderBuilder::new()
            .wallet(alice.clone())
            .connect(RPC_URL)
            .await
            .unwrap();

        // token
        let (_, owner_token, ..) = init(owner_provider.clone()).await.unwrap();
        let owner_balance = owner_token.balanceOf(owner.address()).call().await.unwrap();
        dbg!(&owner_balance);

        let (_, alice_token, ..) = init(alice_provider.clone()).await.unwrap();
        let alice_balance = alice_token.balanceOf(alice.address()).call().await.unwrap();
        dbg!(&alice_balance);

        owner_token
            .transfer(alice.address(), U256::from(10))
            .send()
            .await
            .unwrap()
            .watch()
            .await
            .unwrap();

        let owner_balance = owner_token.balanceOf(owner.address()).call().await.unwrap();
        dbg!(&owner_balance);
        let alice_balance = alice_token.balanceOf(alice.address()).call().await.unwrap();
        dbg!(&alice_balance);
    }

    #[traced_test]
    #[tokio::test]
    async fn test_accounts() {
        let accounts = read_accounts().unwrap();

        let owner = accounts[0].clone();
        let owner_provider = ProviderBuilder::new()
            .wallet(owner.clone())
            .connect(RPC_URL)
            .await
            .unwrap();

        for acc in &accounts {
            println!(
                "Balance({}): {}",
                acc.address(),
                owner_provider.get_balance(acc.address()).await.unwrap()
            );
        }

        let (bridge_contract_owner, ..) = init(owner_provider.clone()).await.unwrap();

        // текущее значение
        // let value = Bridge_contract_owner.get().call().await.unwrap();
        // assert_eq!(value, Uint::ZERO);

        // Устанавливаем новое значение
        // let new_value = Uint::from(100);
        // Bridge_contract_owner
        //     .set(new_value)
        //     .send()
        //     .await
        //     .unwrap()
        //     .watch()
        //     .await
        //     .unwrap();

        // Проверяем что значение изменилось
        // let value = Bridge_contract_owner.get().call().await.unwrap();
        // assert_eq!(value, new_value);

        // Смотрим видно ли значение для alice
        let alice = accounts[1].clone();
        let alice_provider = ProviderBuilder::new()
            .wallet(alice.clone())
            .connect(RPC_URL)
            .await
            .unwrap();

        let (bridge_contract_alice, ..) = init(alice_provider.clone()).await.unwrap();
        // let value = Bridge_contract_alice.get().call().await.unwrap();
        // assert_eq!(value, new_value); // Значение единое для всех

        //
        let watch_send_event = bridge_contract_owner.Send_filter().watch().await.unwrap();
        let event_handle = tokio::spawn(async {
            use futures_util::StreamExt;

            let mut k = watch_send_event.into_stream();
            while let Some(Ok((v, l))) = k.next().await {
                println!("{v:#?}");
                dbg!(l);
            }
        });

        //

        let owner_old_balance = owner_provider.get_balance(owner.address()).await.unwrap();
        let alice_old_balance = alice_provider.get_balance(alice.address()).await.unwrap();

        println!("Balances:");
        println!("owner: {}", owner_old_balance);
        println!("alice: {}", alice_old_balance);

        let send_amount = Uint::from(1_000_000);
        let tx_deposit = bridge_contract_alice
            .deposite(alice.address())
            .value(send_amount)
            .send()
            .await
            .unwrap()
            .watch()
            .await
            .unwrap();
        let owner_balance = owner_provider.get_balance(owner.address()).await.unwrap();
        let alice_balance = alice_provider.get_balance(alice.address()).await.unwrap();
        println!("Balances:");
        println!("owner: {}", owner_balance);
        println!("alice: {}", alice_balance);

        assert_eq!(owner_balance, owner_old_balance + send_amount);
        assert!(alice_balance < alice_old_balance - send_amount);

        // let last_block = owner_provider
        //     .get_block(BlockId::Number(BlockNumberOrTag::Latest))
        //     .await
        //     .unwrap()
        //     .unwrap();

        // let last_block_header = last_block.header;

        let tx_receipt = owner_provider
            .get_transaction_receipt(tx_deposit)
            .await
            .unwrap()
            .unwrap();

        // owner_provider.get;
        dbg!(&tx_receipt);

        // bridge_contract_owner
        //     .check_up()
        //     .send()
        //     .await
        //     .unwrap()
        //     .watch()
        //     .await
        //     .unwrap();
        // bridge_contract_alice
        //     .check_up()
        //     .send()
        //     .await
        //     .unwrap()
        //     .watch()
        //     .await
        //     .unwrap();

        event_handle.abort();
        return;
        // let transport = web3::transports::Http::new("http://localhost:8545").unwrap();
        // let web3 = web3::Web3::new(transport);

        // // address: 0xd70e196eaea04eb065f8ad1acdc67c7ece43b7d9
        // // private key: cc4a7682c00703a233acab918d2e92dcdfb828663b4a8c84d4b561f6d3277ab3

        // // let acc = LocalWallet::decrypt_keystore("data/keystore/UTC--2025-05-12T06-52-14.764238799Z--d70e196eaea04eb065f8ad1acdc67c7ece43b7d9", "").unwrap();

        // let acc: Wallet<_> =
        //     Wallet::from_str("cc4a7682c00703a233acab918d2e92dcdfb828663b4a8c84d4b561f6d3277ab3")
        //         .unwrap();

        // println!("Calling accounts.");
        // let mut accounts = web3.eth().accounts().await.unwrap();

        // dbg!(acc.address());
        // dbg!(hex::encode(acc.signer().to_bytes().to_vec()));
        // println!("Accounts: {:?}", accounts);

        // return;

        // dbg!(&accounts);

        // let alice = accounts[0].clone();
        // let bob = accounts[1].clone();

        // dbg!(&alice, web3.eth().balance(alice, None).await);
        // dbg!(&bob, web3.eth().balance(bob, None).await);

        // dbg!(&web3.eth().block_number().await);

        // let tx = TransactionRequest {
        //     from: alice,
        //     to: Some(bob),
        //     value: Some(1_000_000.into()),
        //     // gas: Some(100_000.into()),
        //     ..TransactionRequest::default()
        // };

        // let t = web3.eth().send_transaction(tx).await.unwrap();

        // dbg!(&alice, web3.eth().balance(alice, None).await);
        // dbg!(&bob, web3.eth().balance(bob, None).await);

        // dbg!(
        //     web3.eth()
        //         .transaction(web3::types::TransactionId::Hash(t))
        //         .await
        // );

        // // Ожидаем подтверждения транзакции
        // loop {
        //     let receipt = web3.eth().transaction_receipt(t).await.unwrap();
        //     dbg!(&receipt);
        //     if receipt.is_some() {
        //         break;
        //     }
        // }
        // // web3.eth().block_with_txs(block)

        // let block_number = web3.eth().block_number().await.unwrap();
        // dbg!(&block_number);

        // dbg!(
        //     &web3
        //         .eth()
        //         .block_with_txs(web3::types::BlockId::Number(
        //             web3::types::BlockNumber::Finalized
        //         ))
        //         .await
        //         .unwrap()
        // );
        // dbg!(
        //     &web3
        //         .eth()
        //         .block_with_txs(web3::types::BlockId::Number(
        //             web3::types::BlockNumber::Latest
        //         ))
        //         .await
        //         .unwrap()
        // );
        // dbg!(
        //     &web3
        //         .eth()
        //         .block_with_txs(web3::types::BlockId::Number(
        //             web3::types::BlockNumber::Earliest
        //         ))
        //         .await
        //         .unwrap()
        // );
        // dbg!(
        //     &web3
        //         .eth()
        //         .block_with_txs(web3::types::BlockId::Number(
        //             web3::types::BlockNumber::Number(block_number)
        //         ))
        //         .await
        //         .unwrap()
        // );
    }

    #[traced_test]
    #[tokio::test]
    async fn test_find_bridge() {
        let accounts = read_accounts().unwrap();

        let owner_signer = accounts[0].clone();
        let owner_provider = ProviderBuilder::new()
            .wallet(owner_signer.clone())
            .connect(RPC_URL)
            .await
            .unwrap();

        let (bridge_contract_owner, ..) = init(owner_provider.clone()).await.unwrap();

        let bridge_address = *bridge_contract_owner.address();
        dbg!(bridge_address);

        // let latest_block = owner_provider.get_block_number().await.unwrap();
        let filter = Filter::new().from_block(0).address(bridge_address);
        // .event_signature(Bridge_contract_owner.);

        let logs = owner_provider.get_logs(&filter).await.unwrap();
        for log in &logs {
            if log.topic0() != Some(&Bridge::Send::SIGNATURE_HASH) {
                continue;
            }
            dbg!(&log.block_number);
            let t: Bridge::Send = log.log_decode().unwrap().inner.data;
            // let t = Bridge::BridgeEvents::decode_log(&log.inner);
            dbg!(t);
            // println!("log: {log:#?}");
        }

        // let mut block_number = 0;

        // let t = owner_provider
        //     .get_code_at(*Bridge_contract_owner.address())
        //     // (*Bridge_contract_owner.address(), U256::from(0))
        //     .await
        //     .unwrap();
        // dbg!(&t);

        // let watch_send_event = Bridge_contract_owner
        //     .Send_filter()
        //     .from_block(0)
        //     // .to_block(0)
        //     .watch()
        //     .await
        //     .unwrap();

        // use futures_util::StreamExt;
        // let mut k = watch_send_event.into_stream();
        // while let Some(Ok((v, l))) = k.next().await {
        //     println!("{v:#?}");
        //     dbg!(l);
        //     break;
        // }

        return;
        // loop {
        //     let Some(block) = owner_provider
        //         .get_block_by_number(BlockNumberOrTag::Number(block_number))
        //         .await
        //         .unwrap()
        //     else {
        //         break;
        //     };

        //     // let t = owner_provider.nb.await;
        //     // dbg!(&block);

        //     // let d = block.try_convert_transactions();
        //     // Bridge_contract_owner.event_filter()

        //     block_number += 1;
        // }

        //
    }
}
