#[cfg(test)]
mod tests {
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
}
