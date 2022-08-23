use anyhow::{Result, bail};
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
            Err(error) => bail!(error.to_string())
        };
        match self.queue.write() {
            Ok(mut queue) => match queue.insert(group, (voice_channel, last_position)) {
                Some(_) => bail!("Group {group} already in queue"),
                None => Ok(())
            },
            Err(error) => bail!(error.to_string())
        }
    }

    /// Returns the next group in the help queue.
    pub async fn next(&self, helper: String) -> Result<(Group, VoiceChannel)> {
        let next = match self.queue.read() {
            Ok(queue) => {
                let aux_queue = queue.clone();
                match aux_queue.iter().min_by(|a, b| a.1.1.cmp(&b.1.1)) {
                    Some(next) => *next.0,
                    None => bail!("No group in queue")
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
            },
            Err(error) => bail!(error.to_string())
        }
    }
    
    /// Removes a group from the help queue.
    async fn remove(&self, group: Group) -> Result<(Group, VoiceChannel)> {
        println!("Removing group {}", group);
        match self.queue.write().unwrap().remove(&group) {
            Some((voice_channel, _)) => Ok((group, voice_channel)),
            None => bail!("Group not in queue")
        }
    }
}

// TODO: Solve 'Cannot start a runtime from within a runtime. This happens 
// because a function (like `block_on`) attempted to block the current thread 
// while the thread is being used to drive asynchronous tasks.'
#[cfg(test)]
mod help_queue_tests {
    use super::*;

    #[tokio::test]
    async fn test01() {
        let queue = HelpQueue::new(ServerArguments::default()).expect("Error creating the help queue");

        assert!(queue.is_empty().is_ok());
        assert!(queue.is_empty().unwrap());
    }

    #[tokio::test]
    async fn test02() {
        let queue = HelpQueue::new(ServerArguments::default()).expect("Error creating the help queue");

        queue.enqueue(1, 887022804183175188).await.expect("Error creating the help queue");

        assert!(queue.len().is_ok());
        assert_eq!(queue.len().unwrap(), 1);
        assert!(queue.is_empty().is_ok());
        assert!(!queue.is_empty().unwrap());
    }

    #[tokio::test]
    async fn test03() {
        let queue = HelpQueue::new(ServerArguments::default()).expect("Error creating the help queue");
        queue.enqueue(1, 887022804183175188).await.expect("Error enqueueing help");

        let expected_result = queue.next("Ivan".to_string()).await;

        if let Ok((group, voice_channel)) = expected_result {
            assert_eq!(group,1);
            assert_eq!(voice_channel, 887022804183175188);
        } 
    }

    #[tokio::test]
    async fn test04() {
        let queue = HelpQueue::new(ServerArguments::default()).expect("Error creating the help queue");

        let expected_result = queue.next("Ivan".to_string()).await;

        assert!(expected_result.is_err());
    }

    #[tokio::test]
    async fn test05() {
        let queue = HelpQueue::new(ServerArguments::default()).expect("Error creating the help queue");

        let expected_result = queue.clear().await;

        assert!(expected_result.is_ok());
        assert!(queue.is_empty().is_ok());
        assert!(queue.is_empty().unwrap());
    }
}
