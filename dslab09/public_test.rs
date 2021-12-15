#[cfg(test)]
mod tests {
    use std::time::Duration;

    use async_channel::{unbounded, bounded};
    use ntest::timeout;

    use crate::solution::{
        register_node, register_store, CyberStore2047, Node, Product, ProductType, Transaction,
        TransactionMessage, TwoPhaseResult, ProductPriceQuery, ProductPrice,
    };
    use executor::{System, ModuleRef};
    use uuid::Uuid;

    #[tokio::test]
    #[timeout(300)]
    async fn transaction_with_two_nodes_completes() {
        // Given:
        let mut system = System::new().await;
        let (transaction_done_tx, transaction_done_rx) = unbounded();
        let products = vec![Product {
            identifier: Uuid::new_v4(),
            pr_type: ProductType::Electronics,
            price: 180,
        }];
        let node0 = register_node(&mut system, Node::new(products.clone())).await;
        let node1 = register_node(&mut system, Node::new(products)).await;
        let cyber_store =
            register_store(&mut system, CyberStore2047::new(vec![node0, node1])).await;

        // When:
        cyber_store
            .send(TransactionMessage {
                transaction: Transaction {
                    pr_type: ProductType::Electronics,
                    shift: -50,
                },
                completed_callback: Box::new(|result| {
                    Box::pin(async move {
                        transaction_done_tx.send(result).await.unwrap();
                    })
                }),
            })
            .await;

        // Then:
        assert_eq!(
            TwoPhaseResult::Ok,
            transaction_done_rx.recv().await.unwrap()
        );
        system.shutdown().await;
    }

    #[tokio::test]
    #[timeout(300)]
    async fn forbidden_transaction_with_two_nodes_is_aborted() {
        // Given:
        let cheap_product = Product {
            identifier: Uuid::new_v4(),
            pr_type: ProductType::Electronics,
            price: 1,
        };

        let mut system = System::new().await;
        let (transaction_done_tx, transaction_done_rx) = unbounded();
        let products1 = vec![Product {
            identifier: Uuid::new_v4(),
            pr_type: ProductType::Electronics,
            price: 180,
        }];
        let products0 = vec![cheap_product];
        let node0 = register_node(&mut system, Node::new(products0)).await;
        let node1 = register_node(&mut system, Node::new(products1)).await;
        let cyber_store =
            register_store(&mut system, CyberStore2047::new(vec![node0.clone(), node1.clone()])).await;

        // When:
        cyber_store
            .send(TransactionMessage {
                transaction: Transaction {
                    pr_type: ProductType::Electronics,
                    shift: -50,
                },
                completed_callback: Box::new(|result| {
                    Box::pin(async move {
                        transaction_done_tx.send(result).await.unwrap();
                    })
                }),
            })
            .await;

        // Then:

        println!("Price now: {}", send_query(&node0, cheap_product.identifier).await.0.unwrap());

        assert_eq!(
            TwoPhaseResult::Abort,
            transaction_done_rx.recv().await.unwrap()
        );
        system.shutdown().await;
    }

    async fn send_query(node: &ModuleRef<Node>, product_ident: Uuid) -> ProductPrice {
        let (result_sender, result_receiver) = bounded::<ProductPrice>(1);
        node.send(ProductPriceQuery {
            product_ident,
            result_sender,
        })
            .await;
        result_receiver.recv().await.unwrap()
    }

    #[tokio::test]
    #[timeout(300)]
    async fn deny_transaction() {
        // Given:
        let mut system = System::new().await;
        let (transaction_done_tx, transaction_done_rx) = unbounded();
        let laptop_id = Uuid::new_v4();
        let initial_laptop_price = 100;
        let products = vec![
            Product {
                identifier: laptop_id,
                pr_type: ProductType::Electronics,
                price: initial_laptop_price,
            },
        ];
        let node = register_node(&mut system, Node::new(products)).await;
        let cyber_store =
            register_store(&mut system, CyberStore2047::new(vec![node.clone()])).await;

        // Increase prices of electronics:
        let electronics_price_shift = -100;
        cyber_store
            .send(TransactionMessage {
                transaction: Transaction {
                    pr_type: ProductType::Electronics,
                    shift: electronics_price_shift,
                },
                completed_callback: Box::new(|result| {
                    Box::pin(async move {
                        transaction_done_tx.send(result).await.unwrap();
                    })
                }),
            })
            .await;
        assert_eq!(Ok(TwoPhaseResult::Abort), transaction_done_rx.recv().await);

        // Check the new price of the latop:
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(
            initial_laptop_price,
            send_query(&node, laptop_id).await.0.unwrap()
        );

        println!("System can execute a simple transaction!");
        system.shutdown().await;
    }

    #[tokio::test]
    #[timeout(3000)]
    async fn overflow() {
        // Given:
        let mut system = System::new().await;
        let (transaction_done_tx, transaction_done_rx) = unbounded();
        let products = vec![Product {
            identifier: Uuid::new_v4(),
            pr_type: ProductType::Electronics,
            price: u64::MAX,
        }];
        let node0 = register_node(&mut system, Node::new(products.clone())).await;
        let node1 = register_node(&mut system, Node::new(products)).await;
        let cyber_store =
            register_store(&mut system, CyberStore2047::new(vec![node0, node1])).await;

        // When:
        cyber_store
            .send(TransactionMessage {
                transaction: Transaction {
                    pr_type: ProductType::Electronics,
                    shift: 1,
                },
                completed_callback: Box::new(|result| {
                    Box::pin(async move {
                        transaction_done_tx.send(result).await.unwrap();
                    })
                }),
            })
            .await;

        // Then:
        assert_eq!(
            TwoPhaseResult::Abort,
            transaction_done_rx.recv().await.unwrap()
        );
        system.shutdown().await;
    }

    #[tokio::test]
    #[timeout(3000)]
    async fn overflow2() {
        // Given:
        let mut system = System::new().await;
        let (transaction_done_tx, transaction_done_rx) = unbounded();
        let products = vec![Product {
            identifier: Uuid::new_v4(),
            pr_type: ProductType::Electronics,
            price: u64::MAX,
        }];
        let node0 = register_node(&mut system, Node::new(products.clone())).await;
        let node1 = register_node(&mut system, Node::new(products)).await;
        let cyber_store =
            register_store(&mut system, CyberStore2047::new(vec![node0, node1])).await;

        // When:
        cyber_store
            .send(TransactionMessage {
                transaction: Transaction {
                    pr_type: ProductType::Electronics,
                    shift: i32::MIN,
                },
                completed_callback: Box::new(|result| {
                    Box::pin(async move {
                        transaction_done_tx.send(result).await.unwrap();
                    })
                }),
            })
            .await;

        // Then:
        assert_eq!(
            TwoPhaseResult::Ok,
            transaction_done_rx.recv().await.unwrap()
        );
        system.shutdown().await;
    }
}
