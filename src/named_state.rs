use std::collections::{HashMap};
use std::hash::Hash;
use std::sync::{Arc, Mutex, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct NamedStateGuard<N: Hash + Eq + PartialEq + Clone> {
    holder: Weak<NamedStateHolder<N>>,
    name: N,
    id: u64,
}

impl<N: Hash + Eq + PartialEq + Clone> Drop for NamedStateGuard<N> {
    fn drop(&mut self) {
        if let Some(holder) = self.holder.upgrade() {
            holder.release_state(self.name.clone(), self.id);
        }
    }
}

struct State<N: Hash + Eq + PartialEq + Clone> {
    names: HashMap<N, Vec<u64>>,
    id_seq: u64,
}
pub struct NamedStateHolder<N: Hash + Eq + PartialEq + Clone> {
    state: Mutex<State<N>>
}

impl <N: Hash + Eq + PartialEq + Clone> NamedStateHolder<N> {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                names: HashMap::new(),
                id_seq: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
            })
        })
    }

    pub fn new_state(self: &Arc<Self>, name: N) -> NamedStateGuard<N> {
        let mut state = self.state.lock().unwrap();
        let id = state.id_seq;
        state.id_seq += 1;
        let list = state.names.entry(name.clone()).or_insert(vec![]);
        list.push(id);
        NamedStateGuard {
            holder: Arc::downgrade(self),
            name,
            id,
        }
    }

    pub fn has_state(&self, name: N) -> bool {
        let state = self.state.lock().unwrap();
        if let Some(list) = state.names.get(&name) {
            list.len() > 0
        } else {
            false
        }
    }

    pub(crate) fn release_state(self: &Arc<Self>, name: N, id: u64) {
        let mut state = self.state.lock().unwrap();
        let list = state.names.entry(name.clone()).or_insert(vec![]);
        list.retain(|&x| x != id);
    }
}

#[cfg(test)]
mod test {
    use crate::NamedStateHolder;

    #[tokio::test]
    async fn test() {
        let holder = NamedStateHolder::new();

        let holder1 = holder.clone();
        let handle = tokio::spawn(async move {
            let _guard1 = holder1.new_state("test".to_string());
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        });
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(holder.has_state("test".to_string()));
        handle.await.unwrap();
        assert!(!holder.has_state("test".to_string()));

        let holder1 = holder.clone();
        let handle = tokio::spawn(async move {
            let _guard1 = holder1.new_state("test".to_string());
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        });

        let holder1 = holder.clone();
        let handle2 = tokio::spawn(async move {
            let _guard1 = holder1.new_state("test".to_string());
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        });
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        assert!(holder.has_state("test".to_string()));
        handle.await.unwrap();
        assert!(holder.has_state("test".to_string()));
        handle2.await.unwrap();
        assert!(!holder.has_state("test".to_string()));
    }
}
