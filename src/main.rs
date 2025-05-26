use crate::contracts::{Bridge, DemoERC20, ExmERC20, TestERC20};
use accounts::Signer;
use alloy::primitives::U256;
use eyre::Result;

pub(crate) mod accounts;
pub(crate) mod console;
pub(crate) mod contracts;

pub const RPC_URL: &str = "http://localhost:8545";

#[tokio::main]
async fn main() -> Result<()> {
    init().await.map(|_| ())
}

async fn init() -> Result<Vec<Signer>> {
    let accounts = accounts::read_accounts().await?;

    let owner = &accounts[0];
    let alice = &accounts[1];
    let bob = &accounts[2];

    let _bridge = contract_provider!(Bridge, owner);
    let demo_erc = contract_provider!(DemoERC20, owner);
    let test_erc = contract_provider!(TestERC20, alice);
    let exm_erc = contract_provider!(ExmERC20, bob);

    let users = accounts[..3].to_vec();

    let addrs = users.iter().map(|v| v.address()).collect::<Vec<_>>();
    token_fund!(demo_erc, addrs);
    token_fund!(test_erc, addrs);
    token_fund!(exm_erc, addrs);

    Ok(accounts)
}

#[cfg(test)]
mod tests {
    use alloy::primitives::U256;
    use tracing::debug;
    use tracing_test::traced_test;

    use crate::{console::watch_logs, contract_provider, contracts::Bridge, init};

    /// ETH
    mod test_eth {

        mod deposite {
            use alloy::{primitives::U256, providers::Provider};
            use tokio::sync::mpsc;
            use tracing::info;
            use tracing_test::traced_test;

            use crate::{
                console::{watch_deposit_event, watch_logs},
                contract_provider,
                contracts::Bridge::{self},
                init, provider,
            };

            #[tokio::test]
            #[traced_test]
            async fn deposit() {
                let acc = init().await.unwrap();

                let console_handle = watch_logs(acc[0].clone()).await.unwrap();

                // Мониторинг событий пополнения депозита
                let (sender, mut rec_deposit_events) = mpsc::channel(10);
                let deposit_handle = watch_deposit_event(acc[0].clone(), sender).await.unwrap();
                let min_amount = U256::from(10).pow(U256::from(10));

                for user in acc.iter().skip(1).take(2) {
                    let user_address = user.address();
                    let provider = provider!(user);

                    let bridge = contract_provider!(Bridge, user);
                    let bridge_address = *bridge.address();

                    let old_balance = provider.get_balance(user_address).await.unwrap();
                    let old_bridge_balance = provider.get_balance(bridge_address).await.unwrap();
                    info!("user balance: {old_balance}");
                    info!("bridge balance: {old_bridge_balance}");

                    info!("Перевод l1=>L2 {min_amount}");
                    let tx = bridge
                        .deposit(user_address)
                        .value(min_amount)
                        .send()
                        .await
                        .unwrap()
                        .watch()
                        .await
                        .unwrap();
                    info!("tx: {:?}", tx);

                    let new_balance = provider.get_balance(user_address).await.unwrap();
                    info!(
                        "user {user_address}\n\
                        old: {old_balance}\n\
                        new: {new_balance}"
                    );
                    let new_bridge_balance = provider.get_balance(bridge_address).await.unwrap();
                    info!(
                        "bridge {bridge_address}\n\
                        old: {old_bridge_balance}\n\
                        new: {new_bridge_balance}"
                    );

                    assert!(old_balance - new_balance > min_amount);
                    // Переведеные деньги должны осесть на балансе моста
                    assert_eq!(new_bridge_balance - old_bridge_balance, min_amount);
                }

                let tx1 = rec_deposit_events.recv().await.unwrap();
                let address = acc[1].address();
                assert_eq!(tx1.from, address);
                assert_eq!(tx1.to, address);
                assert_eq!(tx1.value, 1);

                let tx2 = rec_deposit_events.recv().await.unwrap();
                let address = acc[2].address();
                assert_eq!(tx2.from, address);
                assert_eq!(tx2.to, address);
                assert_eq!(tx2.value, 1);

                deposit_handle.abort();
                console_handle.abort();
            }

            #[tokio::test]
            #[traced_test]
            async fn deposit_err_min_decimal() {
                let acc = init().await.unwrap();
                let amount = U256::from(10).pow(U256::from(10)) - U256::from(1);

                let user = &acc[1];
                let bridge = contract_provider!(Bridge, user);
                assert!(
                    bridge
                        .deposit(user.address())
                        .value(amount)
                        .send()
                        .await
                        .is_err()
                );
            }
        }

        mod withdraw {
            use alloy::{
                consensus::constants::ETH_TO_WEI, network::TransactionBuilder, primitives::U256,
                providers::Provider, rpc::types::TransactionRequest,
            };

            use tracing::{debug, info};
            use tracing_test::traced_test;

            use crate::{
                contract_provider,
                contracts::Bridge::{self},
                init, provider,
            };

            #[tokio::test]
            #[traced_test]
            async fn withdrow() {
                let acc = init().await.unwrap();

                // Запрос на вывод отправляется от owner
                let owner = acc[0].clone();
                let owner_bridge = contract_provider!(Bridge, owner);
                let owner_provider = provider!(owner);

                // Проверка достаточно ли баланса на bridge для вывода

                let bridge_address = *owner_bridge.address();
                let bridge_balance = owner_provider.get_balance(bridge_address).await.unwrap();
                let need_balance = U256::from(10).pow(U256::from(10)) * U256::from(2);
                if bridge_balance < need_balance {
                    dbg!(owner_provider.get_balance(owner.address()).await.unwrap());

                    debug!("Не достаточно средств на мосте {bridge_balance} < {need_balance}");

                    info!("Пополняем баланс");

                    let tx = owner_provider
                        .send_transaction(
                            TransactionRequest::default()
                                .with_from(owner.address())
                                .with_to(bridge_address)
                                .with_value(U256::from(ETH_TO_WEI)),
                        )
                        .await
                        .unwrap()
                        .watch()
                        .await
                        .unwrap();
                    info!("Баланс моста пополнен на 1 eth. tx:{tx:#?} ");
                }

                for user in acc.iter().skip(1).take(2) {
                    let address = user.address();
                    let old_balance = owner_provider.get_balance(address).await.unwrap();

                    let bridge = contract_provider!(Bridge, user);
                    let old_bridge_balance =
                        owner_provider.get_balance(bridge_address).await.unwrap();

                    let amount = U256::from(10_u64.pow(14)); // 0.0001 ETH

                    let old_withdraw_amount = bridge.status_withdraw().call().await.unwrap();
                    info!("Разрешаем на вывод 0.00000001 ETH для {address}");
                    let tx = owner_bridge
                        .request_withdraw(address, 10000) // 1 = 0.0001 ETH
                        .send()
                        .await
                        .unwrap()
                        .watch()
                        .await
                        .unwrap();
                    info!("tx: {tx:#?}");

                    // Баланс моста не должен измениться
                    assert_eq!(
                        old_bridge_balance,
                        owner_provider.get_balance(bridge_address).await.unwrap()
                    );

                    let new_withdraw_amount = bridge.status_withdraw().call().await.unwrap();
                    assert_eq!(
                        new_withdraw_amount - old_withdraw_amount,
                        amount,
                        "old: {old_withdraw_amount}\n\
                        new: {new_withdraw_amount}"
                    );

                    let tx = bridge
                        .withdraw()
                        .send()
                        .await
                        .unwrap()
                        .watch()
                        .await
                        .unwrap();
                    info!("tx: {tx:#?}");

                    assert_eq!(bridge.status_withdraw().call().await.unwrap(), U256::ZERO);

                    let new_bridge_balance =
                        owner_provider.get_balance(bridge_address).await.unwrap();
                    assert_eq!(
                        old_bridge_balance - amount,
                        new_bridge_balance,
                        "old: {old_bridge_balance}\n\
                        new: {new_bridge_balance}"
                    );
                    let new_balance = owner_provider.get_balance(address).await.unwrap();
                    assert!(
                        old_balance - amount < new_balance,
                        "old: {old_balance}\n\
                        new: {new_balance}"
                    );
                }
            }

            #[tokio::test]
            #[traced_test]
            async fn withdrow_err_not_onwer() {
                let acc = init().await.unwrap();

                for user in acc.into_iter().skip(1).take(2) {
                    let bridge = contract_provider!(Bridge, user);

                    let res = bridge.request_withdraw(user.address(), 1).call().await;
                    assert!(res.is_err(), "{res:#?}");
                }
            }

            #[tokio::test]
            #[traced_test]
            async fn withdrow_err_without_approval() {
                let acc = init().await.unwrap();

                for user in acc.into_iter().skip(1).take(2) {
                    let bridge = contract_provider!(Bridge, user);

                    if bridge.status_withdraw().call().await.unwrap() != U256::ZERO {
                        continue;
                    }

                    let res = bridge.withdraw().send().await;
                    assert!(res.is_err(), "{res:#?}");
                }
            }
        }
    }

    /// ERC20
    mod tests_erc {
        use alloy::primitives::U256;

        fn calc_min_amount(decimal: u8) -> U256 {
            if decimal > 8 {
                U256::from(10_u64.pow(decimal as u32 - 8))
            } else {
                U256::from(1)
            }
        }

        /// Создание моста для (ERC20)
        mod tests_create_bridge {
            use alloy::{
                consensus::constants::ETH_TO_WEI,
                primitives::{Address, U256},
                providers::{Provider, ProviderBuilder},
            };
            use rand::random;
            use tracing::{info, warn};
            use tracing_test::traced_test;

            use crate::{
                RPC_URL, contract_provider,
                contracts::{Bridge, DemoERC20, ExmERC20, TestERC20},
                init,
            };

            // (ERC20) попытка подключить без коммиссии или с недостаточной комиссией
            #[tokio::test]
            #[traced_test]
            async fn err_commission() {
                let acc = init().await.unwrap();

                let owner = &acc[1];
                let provider = ProviderBuilder::new()
                    .wallet(owner.clone())
                    .connect(RPC_URL)
                    .await
                    .unwrap();

                let new_token = DemoERC20::deploy(provider).await.unwrap();
                let new_token_address = *new_token.address();
                let bridge = contract_provider!(Bridge, owner);

                if bridge
                    .exist_bridge_erc20(new_token_address)
                    .call()
                    .await
                    .unwrap()
                {
                    warn!("Контракт существует {new_token_address}");
                    return;
                }

                assert!(
                    bridge
                        .create_bridge_erc20(new_token_address)
                        .send()
                        .await
                        .is_err()
                );

                assert!(
                    bridge
                        .create_bridge_erc20(new_token_address)
                        .value(U256::from(ETH_TO_WEI - 1))
                        .send()
                        .await
                        .is_err()
                );
            }

            // (ERC20) попытка подключить несуществующего токен
            #[tokio::test]
            #[traced_test]
            async fn err_invalid_token() {
                let acc = init().await.unwrap();

                let token_address = Address::from_slice(&random::<[u8; 20]>());

                for user in &acc[..2] {
                    let bridge = contract_provider!(Bridge, user);
                    let exi = bridge
                        .exist_bridge_erc20(token_address)
                        .call()
                        .await
                        .unwrap();
                    if exi {
                        continue;
                    }

                    assert!(
                        bridge
                            .create_bridge_erc20(token_address)
                            .value(U256::from(ETH_TO_WEI))
                            .send()
                            .await
                            .is_err()
                    );
                }
            }

            // (ERC20) Успешное подключение ERC20 токена
            #[tokio::test]
            #[traced_test]
            async fn success() {
                let acc = init().await.unwrap();

                let [owner, alice, bob] = acc[..3].to_vec().try_into().unwrap();

                for (user, token_address) in [
                    (
                        owner.clone(),
                        *contract_provider!(DemoERC20, owner).address(),
                    ),
                    (
                        alice.clone(),
                        *contract_provider!(TestERC20, alice).address(),
                    ),
                    (bob.clone(), *contract_provider!(ExmERC20, bob).address()),
                ] {
                    let bridge = contract_provider!(Bridge, user);
                    let bridge_address = *bridge.address();
                    let provider = ProviderBuilder::new()
                        .wallet(user.clone())
                        .connect(RPC_URL)
                        .await
                        .unwrap();

                    if bridge
                        .exist_bridge_erc20(token_address)
                        .call()
                        .await
                        .unwrap()
                    {
                        info!("Моста для токена {token_address} существует ");
                        continue;
                    }

                    let amount = U256::from(ETH_TO_WEI);
                    let old_bridge_balance = provider.get_balance(bridge_address).await.unwrap();
                    let old_user_balance = provider.get_balance(user.address()).await.unwrap();

                    let tx = bridge
                        .create_bridge_erc20(token_address)
                        .value(amount)
                        .send()
                        .await
                        .unwrap()
                        .watch()
                        .await
                        .unwrap();
                    info!("Мост для {token_address} создан  Tx {tx:?}");

                    assert!(
                        bridge
                            .exist_bridge_erc20(token_address)
                            .call()
                            .await
                            .unwrap()
                    );

                    let new_bridge_balance = provider.get_balance(bridge_address).await.unwrap();
                    let new_user_balance = provider.get_balance(user.address()).await.unwrap();

                    assert_eq!(old_bridge_balance + amount, new_bridge_balance);
                    assert!(old_user_balance > new_user_balance - amount);
                }
            }
        }

        /// (ERC20) Пополнение баланса на l2
        mod deposit {
            use alloy::{consensus::constants::ETH_TO_WEI, primitives::U256};

            use tokio::sync::mpsc;
            use tracing::{debug, info};
            use tracing_test::traced_test;

            use crate::{
                console::{self, watch_deposit_erc20_event},
                contract_provider,
                contracts::{Bridge, DemoERC20, ExmERC20, TestERC20},
                init,
                tests::tests_erc::calc_min_amount,
                tokens_balans,
            };

            #[tokio::test]
            #[traced_test]
            async fn deposite() {
                let acc = init().await.unwrap();

                let console_handle = console::watch_logs(acc[0].clone()).await.unwrap();

                let owner = acc[0].clone();
                let onwer_bridge = contract_provider!(Bridge, owner);
                let bridge_address = *onwer_bridge.address();
                let demo_token = contract_provider!(DemoERC20, owner);
                let test_token = contract_provider!(TestERC20, owner);
                let exm_token = contract_provider!(ExmERC20, owner);

                let tokens = [
                    *demo_token.address(),
                    *test_token.address(),
                    *exm_token.address(),
                ];
                for token_address in &tokens {
                    if onwer_bridge
                        .exist_bridge_erc20(*token_address)
                        .call()
                        .await
                        .unwrap()
                    {
                        info!("Мост уже существует");
                        continue;
                    }

                    info!("Создание моста для токена {token_address}");
                    let tx = onwer_bridge
                        .create_bridge_erc20(*token_address)
                        .value(U256::from(ETH_TO_WEI))
                        .send()
                        .await
                        .unwrap()
                        .watch()
                        .await
                        .unwrap();
                    info!("Tx {tx:?}");
                }

                let min_amount = [
                    calc_min_amount(demo_token.decimals().call().await.unwrap()),
                    calc_min_amount(test_token.decimals().call().await.unwrap()),
                    calc_min_amount(exm_token.decimals().call().await.unwrap()),
                ];

                // Мониторинг событий пополнения депозита
                let (sender, mut rec_deposit_events) = mpsc::channel(10);
                let deposite_handle = watch_deposit_erc20_event(owner, sender).await.unwrap();

                for user in acc.into_iter().skip(1).take(2) {
                    let user_address = user.address();
                    let user_bridge = contract_provider!(Bridge, user);
                    let user_demo_token = contract_provider!(DemoERC20, user);
                    let user_test_token = contract_provider!(TestERC20, user);
                    let user_exm_token = contract_provider!(ExmERC20, user);

                    info!("Одобрение перевода на кошелёк");
                    user_demo_token
                        .approve(bridge_address, min_amount[0])
                        .send()
                        .await
                        .unwrap()
                        .watch()
                        .await
                        .unwrap();
                    user_test_token
                        .approve(bridge_address, min_amount[1])
                        .send()
                        .await
                        .unwrap()
                        .watch()
                        .await
                        .unwrap();
                    user_exm_token
                        .approve(bridge_address, min_amount[2])
                        .send()
                        .await
                        .unwrap()
                        .watch()
                        .await
                        .unwrap();

                    let old_user_balance = tokens_balans!(
                        user_address,
                        user_demo_token,
                        user_test_token,
                        user_exm_token
                    );
                    let old_bridge_balance = tokens_balans!(
                        bridge_address,
                        user_demo_token,
                        user_test_token,
                        user_exm_token
                    );

                    for (token_address, amount) in tokens.iter().zip(min_amount) {
                        user_bridge
                            .deposit_erc20(*token_address, user_address, amount)
                            .send()
                            .await
                            .unwrap()
                            .watch()
                            .await
                            .unwrap();
                    }

                    for (tx, token) in [
                        rec_deposit_events.recv().await.unwrap(),
                        rec_deposit_events.recv().await.unwrap(),
                        rec_deposit_events.recv().await.unwrap(),
                    ]
                    .iter()
                    .zip(tokens)
                    {
                        assert_eq!(tx.token_address, token);
                        assert_eq!(tx.from, user_address);
                        assert_eq!(tx.to, user_address);
                        assert_eq!(tx.value, U256::from(1));
                    }

                    let new_user_balance = tokens_balans!(
                        user_address,
                        user_demo_token,
                        user_test_token,
                        user_exm_token
                    );
                    let new_bridge_balance = tokens_balans!(
                        bridge_address,
                        user_demo_token,
                        user_test_token,
                        user_exm_token
                    );

                    for ((old, new), amount) in old_user_balance
                        .iter()
                        .zip(&new_user_balance)
                        .zip(&min_amount)
                    {
                        assert_eq!(
                            old - new,
                            *amount,
                            "new: {new}, old: {old}, amount: {amount}"
                        );
                    }

                    for ((old, new), amount) in old_bridge_balance
                        .iter()
                        .zip(&new_bridge_balance)
                        .zip(&min_amount)
                    {
                        assert_eq!(
                            new - old,
                            *amount,
                            "new: {new}, old: {old}, amount: {amount}"
                        );
                    }
                }
                console_handle.abort();
                deposite_handle.abort();
            }

            #[tokio::test]
            #[traced_test]
            async fn deposit_err_min_decimal() {
                let acc = init().await.unwrap();
                let alice = acc[1].to_owned();

                let demo_token = contract_provider!(DemoERC20, alice);
                let demo_address = *demo_token.address();

                let bridge = contract_provider!(Bridge, alice);
                let bridge_address = *bridge.address();

                if !bridge
                    .exist_bridge_erc20(demo_address)
                    .call()
                    .await
                    .unwrap()
                {
                    bridge
                        .create_bridge_erc20(demo_address)
                        .value(U256::from(ETH_TO_WEI))
                        .send()
                        .await
                        .unwrap()
                        .watch()
                        .await
                        .unwrap();
                }

                let decimals = demo_token.decimals().call().await.unwrap();
                info!("Decimals: {decimals}");
                assert!(decimals > 8);

                let min = U256::from(10_u64.pow(decimals as u32 - 8) + 1);
                demo_token
                    .approve(bridge_address, min)
                    .send()
                    .await
                    .unwrap()
                    .watch()
                    .await
                    .unwrap();
                let res = bridge
                    .deposit_erc20(demo_address, alice.address(), min)
                    .send()
                    .await;
                debug!("{res:#?}");
                assert!(res.is_err(), "{res:#?}");
            }

            #[tokio::test]
            #[traced_test]
            async fn deposit_err_without_approve() {
                let acc = init().await.unwrap();
                let alice = acc[1].to_owned();

                let demo_token = contract_provider!(DemoERC20, alice);
                let demo_address = *demo_token.address();

                let bridge = contract_provider!(Bridge, alice);
                let bridge_address = *bridge.address();

                if !bridge
                    .exist_bridge_erc20(demo_address)
                    .call()
                    .await
                    .unwrap()
                {
                    bridge
                        .create_bridge_erc20(demo_address)
                        .value(U256::from(ETH_TO_WEI))
                        .send()
                        .await
                        .unwrap()
                        .watch()
                        .await
                        .unwrap();
                }

                let decimals = demo_token.decimals().call().await.unwrap();
                info!("Decimals: {decimals}");
                assert!(decimals > 8);

                let min = U256::from(10_u64.pow(decimals as u32 - 8));
                demo_token
                    .approve(bridge_address, min)
                    .send()
                    .await
                    .unwrap()
                    .watch()
                    .await
                    .unwrap();
                let res = bridge
                    .deposit_erc20(demo_address, alice.address(), min * U256::from(2))
                    .send()
                    .await;
                debug!("{res:#?}");
                assert!(res.is_err(), "{res:#?}");
            }
        }

        mod withdraw {
            use alloy::{consensus::constants::ETH_TO_WEI, primitives::U256};
            use tracing::{debug, info};
            use tracing_test::traced_test;

            use crate::{
                console::{self},
                contract_provider,
                contracts::{Bridge, DemoERC20, ExmERC20, TestERC20},
                init,
                tests::tests_erc::calc_min_amount,
                token_fund, tokens_balans,
            };

            #[tokio::test]
            #[traced_test]
            async fn withdraw() {
                let acc = init().await.unwrap();

                let console_handle = console::watch_logs(acc[0].clone()).await.unwrap();

                let owner = acc[0].clone();
                let owner_address = owner.address();

                let alice = acc[1].clone();
                let alice_address = alice.address();
                debug!("alice: {alice_address:?}");

                let bob = acc[2].clone();
                let bob_address = bob.address();
                debug!("bob: {bob_address:?}");

                let onwer_bridge = contract_provider!(Bridge, owner);
                let bridge_address = *onwer_bridge.address();
                debug!("bridge address: {bridge_address:?}");

                let demo_token = contract_provider!(DemoERC20, owner);
                let test_token = contract_provider!(TestERC20, alice);
                let exm_token = contract_provider!(ExmERC20, bob);

                // Пополнение баланса моста
                {
                    let bridge_address = [bridge_address];
                    token_fund!(demo_token, bridge_address);
                    token_fund!(test_token, bridge_address);
                    token_fund!(exm_token, bridge_address);
                }

                let tokens = [
                    *demo_token.address(),
                    *test_token.address(),
                    *exm_token.address(),
                ];

                // Проверка существования и создание мостов
                for token_address in &tokens {
                    if onwer_bridge
                        .exist_bridge_erc20(*token_address)
                        .call()
                        .await
                        .unwrap()
                    {
                        info!("Мост уже существует");
                        continue;
                    }

                    info!("Создание моста для токена {token_address}");
                    let tx = onwer_bridge
                        .create_bridge_erc20(*token_address)
                        .value(U256::from(ETH_TO_WEI))
                        .send()
                        .await
                        .unwrap()
                        .watch()
                        .await
                        .unwrap();
                    info!("Tx {tx:?}");
                }

                let min_amount = [
                    calc_min_amount(demo_token.decimals().call().await.unwrap()),
                    calc_min_amount(test_token.decimals().call().await.unwrap()),
                    calc_min_amount(exm_token.decimals().call().await.unwrap()),
                ];

                let old_onwer_balance =
                    tokens_balans!(owner_address, demo_token, test_token, exm_token);
                let old_alice_balance =
                    tokens_balans!(alice_address, demo_token, test_token, exm_token);
                let old_bob_balance =
                    tokens_balans!(bob_address, demo_token, test_token, exm_token);
                let old_bridge_balance =
                    tokens_balans!(bridge_address, demo_token, test_token, exm_token);

                for token_address in tokens {
                    for user in acc.iter().skip(1).take(2) {
                        let user_address = user.address();
                        info!(
                            "Owner создаёт заявки на вывод токена {token_address:?} на адресс {user_address}"
                        );
                        let tx = onwer_bridge
                            .request_withdraw_erc20(token_address, user.address(), 1)
                            .send()
                            .await
                            .unwrap()
                            .watch()
                            .await
                            .unwrap();
                        info!("Tx {tx:?}");

                        let bridge = contract_provider!(Bridge, user);
                        let withdraw_balance = bridge
                            .status_withdraw_erc20(token_address)
                            .call()
                            .await
                            .unwrap();
                        assert_ne!(withdraw_balance, U256::ZERO);

                        info!("Выводим токены {token_address:?} из моста на адресс {user_address}");
                        let tx = bridge
                            .withdraw_erc20(token_address)
                            .send()
                            .await
                            .unwrap()
                            .watch()
                            .await
                            .unwrap();
                        info!("Tx {tx:?}");
                    }
                }

                let new_owner_balance =
                    tokens_balans!(owner_address, demo_token, test_token, exm_token);
                let new_alice_balance =
                    tokens_balans!(alice_address, demo_token, test_token, exm_token);
                let new_bob_balance =
                    tokens_balans!(bob_address, demo_token, test_token, exm_token);
                let new_bridge_balance =
                    tokens_balans!(bridge_address, demo_token, test_token, exm_token);

                for (old, new) in [
                    (old_alice_balance, new_alice_balance),
                    (old_bob_balance, new_bob_balance),
                ] {
                    for ((old, new), amount) in old.iter().zip(new).zip(min_amount) {
                        assert_eq!(
                            new - *old,
                            amount,
                            "new: {new}, old: {old}, expected: {amount}"
                        );
                    }
                }

                assert_eq!(old_onwer_balance, new_owner_balance);

                dbg!(&old_bridge_balance, &new_bridge_balance);

                for ((old, new), amount) in old_bridge_balance
                    .iter()
                    .zip(new_bridge_balance)
                    .zip(min_amount)
                {
                    let amount_x2 = amount * U256::from(2);
                    assert_eq!(
                        old - new,
                        amount_x2,
                        "new: {new}, old: {old}, expected: {amount_x2}"
                    );
                }

                console_handle.abort();
            }

            #[tokio::test]
            #[traced_test]
            async fn withdraw_err_not_owner() {
                let acc = init().await.unwrap();

                let alice = acc[1].to_owned();
                let demo_token = contract_provider!(DemoERC20, alice);
                let bridge = contract_provider!(Bridge, alice);

                let res = bridge
                    .request_withdraw_erc20(*demo_token.address(), alice.address(), 1)
                    .send()
                    .await;
                debug!("{res:#?}");
                assert!(res.is_err(), "{res:#?}");
            }
        }
    }

    #[tokio::test]
    #[traced_test]
    async fn convert_decimals() {
        let owner = init().await.unwrap()[0].to_owned();
        let bridge = contract_provider!(Bridge, owner);

        let _handle = watch_logs(owner.clone()).await.unwrap();

        for (input, output) in [
            ((1_234, 0, 0), 1_234),
            ((1_234, 3, 3), 1_234),
            ((1_234, 0, 1), 12_340),
            ((1_234, 0, 3), 1_234_000),
            ((1_234, 1, 3), 123_400),
            ((1_234, 1, 3), 123_400),
            ((1_000, 3, 0), 1),
            ((1_000, 2, 0), 10),
            ((1_000, 2, 1), 100),
            ((1_230, 1, 0), 123),
            ((1_230, 3, 2), 123),
        ] {
            debug!("input: {input:#?}, output: {output}");
            assert_eq!(
                bridge
                    .convert_amount(U256::from(input.0), input.1, input.2)
                    .call()
                    .await
                    .unwrap(),
                U256::from(output),
                "input: {input:#?}, output: {output}"
            );
        }

        for input in [(1_230, 3, 0), (1_230, 2, 0)] {
            let r = bridge
                .convert_amount(U256::from(input.0), input.1, input.2)
                .call()
                .await;
            debug!("{r:#?}");
            assert!(r.is_err(), "{r:#?}");
        }
    }
}
