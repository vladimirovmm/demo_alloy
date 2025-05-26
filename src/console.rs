#![cfg(test)]

use alloy::{providers::Provider, rpc::types::Filter};
use alloy_sol_types::SolEvent;
use eyre::Result;
use futures_util::StreamExt;
use tokio::{sync::mpsc::Sender, task::JoinHandle};

use crate::{
    accounts::Signer,
    contract_provider,
    contracts::{Bridge, DemoERC20, console},
    provider,
};

pub(crate) async fn watch_logs(user: Signer) -> Result<JoinHandle<()>> {
    let filter = Filter::new();
    let provider = provider!(user);

    let handle = tokio::spawn(async move {
        let watch_send_event = provider.watch_logs(&filter).await.unwrap();
        let mut k = watch_send_event.into_stream();

        while let Some(logs) = k.next().await {
            for log in logs {
                let Some(log_topic) = log.topic0() else {
                    continue;
                };

                match *log_topic {
                    Bridge::EventCreateBridge::SIGNATURE_HASH => {
                        let msg = log.log_decode::<Bridge::EventCreateBridge>().unwrap();
                        println!("Создание моста для ERC20");
                        println!("Log: {:#?}", msg.data());
                    }
                    Bridge::EventDeposit::SIGNATURE_HASH => {
                        let msg = log.log_decode::<Bridge::EventDeposit>().unwrap();
                        println!("Перевод ETH с l1=>l2");
                        println!("Log: {:#?}", msg.data());
                    }
                    Bridge::EventDepositRC20::SIGNATURE_HASH => {
                        let msg = log.log_decode::<Bridge::EventDepositRC20>().unwrap();
                        println!("Перевод ERC20 с l1=>l2");
                        println!("Log: {:#?}", msg.data());
                    }

                    DemoERC20::Approval::SIGNATURE_HASH => {
                        let msg = log.log_decode::<DemoERC20::Approval>().unwrap();
                        println!("Log: {:#?}", msg.data());
                    }
                    console::Vote::SIGNATURE_HASH => {
                        let msg = log.log_decode::<console::Vote>().unwrap();
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
    Ok(handle)
}

pub(crate) async fn watch_deposit_event(
    user: Signer,
    sender: Sender<Bridge::EventDeposit>,
) -> Result<JoinHandle<()>> {
    let bridge = contract_provider!(Bridge, user);

    let event_handle = tokio::spawn(async move {
        use futures_util::StreamExt;

        let watch_send_event = bridge.EventDeposit_filter().watch().await.unwrap();
        let mut k = watch_send_event.into_stream();
        while let Some(Ok((event, _l))) = k.next().await {
            sender.send(event).await.unwrap();
        }
    });
    Ok(event_handle)
}

pub(crate) async fn watch_deposit_erc20_event(
    user: Signer,
    sender: Sender<Bridge::EventDepositRC20>,
) -> Result<JoinHandle<()>> {
    let bridge = contract_provider!(Bridge, user);

    let event_handle = tokio::spawn(async move {
        use futures_util::StreamExt;

        let watch_send_event = bridge.EventDepositRC20_filter().watch().await.unwrap();
        let mut k = watch_send_event.into_stream();
        while let Some(Ok((event, _l))) = k.next().await {
            sender.send(event).await.unwrap();
        }
    });
    Ok(event_handle)
}
