use anyhow::{bail, Result};
use indexmap::IndexMap;
use std::sync::{Arc, RwLock};

/// Shorthand for the group number.
type Group = u16;
/// Shorthand for discord's voice channel id.
type VoiceChannel = u64;

/// The help queue.
#[derive(Debug)]
pub struct HelpQueue {
    queue: RwLock<IndexMap<Group, (VoiceChannel, usize)>>,
    // TODO: Implement logger
    // logger
}

impl HelpQueue {
    pub fn new() -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            queue: RwLock::new(IndexMap::new()),
        }))
    }

    /// Pushes a requester to the help queue.
    pub async fn enqueue(&self, group: Group, voice_channel: VoiceChannel) -> Result<()> {
        println!("Enqueueing group {}", group);
        let last_position = match self.len() {
            Ok(position) => position,
            Err(error) => bail!(error.to_string()),
        };
        match self.queue.write() {
            Ok(mut queue) => match queue.insert(group, (voice_channel, last_position)) {
                Some(_) => bail!("Group {group} already in queue"),
                None => Ok(()),
            },
            Err(error) => bail!(error.to_string()),
        }
    }

    /// Returns the next group in the help queue.
    pub async fn next(&self, helper: &str) -> Result<(Group, VoiceChannel)> {
        let next = match self.queue.read() {
            Ok(queue) => {
                let aux_queue = queue.clone();
                match aux_queue.iter().min_by(|a, b| a.1 .1.cmp(&b.1 .1)) {
                    Some(next) => *next.0,
                    None => bail!("No group in queue"),
                }
            }
            Err(error) => bail!(error.to_string()),
        };

        print!("{} helped group {}", helper, next);

        self.remove(next).await
        // TODO: Log help.
    }

    /// Removes the dismisser from the help queue.
    pub async fn dismiss(&self, dismisser: Group) -> Result<(Group, VoiceChannel)> {
        println!("Dismissing group {} help request", dismisser);
        self.remove(dismisser).await
        // TODO: Log dismissal.
    }

    /// Clears the help queue.
    pub async fn clear(&self) -> Result<()> {
        match self.queue.write() {
            Ok(mut queue) => queue.clear(),
            Err(error) => bail!(error.to_string()),
        }
        Ok(())
    }

    /// Returns the length of the help queue.
    pub fn len(&self) -> Result<usize> {
        match self.queue.read() {
            Ok(queue) => Ok(queue.len()),
            Err(error) => bail!(error.to_string()),
        }
    }

    /// Returns whether the queue is empty or not.
    pub fn is_empty(&self) -> Result<bool> {
        match self.queue.read() {
            Ok(queue) => Ok(queue.is_empty()),
            Err(error) => bail!(error.to_string()),
        }
    }

    /// Returns the help queue in order.
    pub fn sorted(&self) -> Result<impl Iterator<Item = Group>> {
        match self.queue.read() {
            Ok(queue) => {
                let aux_queue = queue.clone();
                let sorted_scores = aux_queue
                    .sorted_by(|_, (_, position_1), _, (_, position_2)| position_1.cmp(position_2))
                    .map(|(group, _)| group);
                Ok(sorted_scores)
            }
            Err(error) => bail!(error.to_string()),
        }
    }

    /// Removes a group from the help queue.
    async fn remove(&self, group: Group) -> Result<(Group, VoiceChannel)> {
        println!("Removing group {}", group);
        match self.queue.write().unwrap().remove(&group) {
            Some((voice_channel, _)) => Ok((group, voice_channel)),
            None => bail!("Group not in queue"),
        }
    }
}

// TODO: Solve 'Cannot start a runtime from within a runtime. This happens
// because a function (like `block_on`) attempted to block the current thread
// while the thread is being used to drive asynchronous tasks.'
#[cfg(test)]
mod help_queue_tests {
    use super::*;

    #[test]
    fn test01_help_queue_should_be_empty_when_created() {
        let queue = HelpQueue::new().expect("Error creating the help queue");

        assert!(queue.is_empty().is_ok());
        assert!(queue.is_empty().unwrap());
    }

    #[tokio::test]
    async fn test02_help_queue_should_not_be_empty_after_enqueueing() {
        let queue = HelpQueue::new().expect("Error creating the help queue");

        queue
            .enqueue(1, 887022804183175188)
            .await
            .expect("Error creating the help queue");

        assert!(queue.len().is_ok());
        assert_eq!(queue.len().unwrap(), 1);
        assert!(queue.is_empty().is_ok());
        assert!(!queue.is_empty().unwrap());
    }

    #[tokio::test]
    async fn test03_next_in_queue_should_be_the_last_enqueued() {
        let queue = HelpQueue::new().expect("Error creating the help queue");
        queue
            .enqueue(1, 887022804183175188)
            .await
            .expect("Error enqueueing help");

        let expected_result = queue.next("Ivan").await;

        if let Ok((group, voice_channel)) = expected_result {
            assert_eq!(queue.len().unwrap(), 0);
            assert_eq!(group, 1);
            assert_eq!(voice_channel, 887022804183175188);
        }
    }

    #[tokio::test]
    async fn test04_more_than_one_group_can_request_for_help() {
        let queue = HelpQueue::new().expect("Error creating the help queue");
        queue
            .enqueue(1, 887022804183175188)
            .await
            .expect("Error enqueueing help");
        queue
            .enqueue(2, 887022804183175189)
            .await
            .expect("Error enqueueing help");

        assert_eq!(queue.len().unwrap(), 2);
    }

    #[tokio::test]
    async fn test05_queue_behaves_fifo() {
        let queue = HelpQueue::new().expect("Error creating the help queue");
        queue
            .enqueue(1, 887022804183175188)
            .await
            .expect("Error enqueueing help");
        queue
            .enqueue(2, 887022804183175189)
            .await
            .expect("Error enqueueing help");

        let expected_result = queue.next("Ivan").await;
        let other_expected_result = queue.next("Ivan").await;

        assert_eq!(queue.len().unwrap(), 0);
        if let Ok((group, voice_channel)) = expected_result {
            assert_eq!(group, 1);
            assert_eq!(voice_channel, 887022804183175188);
        }
        if let Ok((group, voice_channel)) = other_expected_result {
            assert_eq!(group, 2);
            assert_eq!(voice_channel, 887022804183175189);
        }
    }

    #[tokio::test]
    async fn test06_cannot_enqueue_the_same_group_twice() {
        let queue = HelpQueue::new().expect("Error creating the help queue");
        queue
            .enqueue(1, 887022804183175188)
            .await
            .expect("Error enqueueing help");

        let expected_result = queue.enqueue(1, 887022804183175189).await;

        assert_eq!(queue.len().unwrap(), 1);
        assert!(expected_result.is_err());
    }

    #[tokio::test]
    async fn test07_there_is_no_next_in_an_empty_queue() {
        let queue = HelpQueue::new().expect("Error creating the help queue");

        let expected_result = queue.next("Ivan").await;

        assert!(expected_result.is_err());
    }

    #[tokio::test]
    async fn test08_queue_is_empty_after_clearing() {
        let queue = HelpQueue::new().expect("Error creating the help queue");
        queue
            .enqueue(1, 887022804183175188)
            .await
            .expect("Error enqueueing help");

        let expected_result = queue.clear().await;

        assert_eq!(queue.len().unwrap(), 0);
        assert!(expected_result.is_ok());
        assert!(queue.is_empty().unwrap());
    }

    #[tokio::test]
    async fn test09_requesters_can_dismiss_their_request() {
        let queue = HelpQueue::new().expect("Error creating the help queue");
        queue
            .enqueue(1, 887022804183175188)
            .await
            .expect("Error enqueueing help");

        let expected_result = queue.dismiss(1).await;

        assert_eq!(queue.len().unwrap(), 0);
        assert!(expected_result.is_ok());
        assert_eq!(expected_result.unwrap(), (1, 887022804183175188));
    }

    #[tokio::test]
    async fn test10_requesters_cannot_dismiss_if_they_did_not_request_for_help() {
        let queue = HelpQueue::new().expect("Error creating the help queue");

        let expected_result = queue.dismiss(2).await;

        assert!(expected_result.is_err());
    }

    #[tokio::test]
    async fn test11_groups_that_requested_for_help_can_be_retrieved_sorted() {
        let queue = HelpQueue::new().expect("Error creating the help queue");
        queue
            .enqueue(1, 887022804183175188)
            .await
            .expect("Error enqueueing help");
        queue
            .enqueue(2, 887022804183175189)
            .await
            .expect("Error enqueueing help");
        queue
            .enqueue(3, 887022804183175190)
            .await
            .expect("Error enqueueing help");

        let expected_result = queue.sorted();

        assert!(expected_result.is_ok());
        assert_eq!(
            expected_result.unwrap().collect::<Vec<u16>>(),
            vec![1, 2, 3]
        );
    }
}
