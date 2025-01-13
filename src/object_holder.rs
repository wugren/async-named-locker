use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use notify_future::NotifyFuture;

pub struct ObjectGuard<T> {
    obj: Option<T>,
    holder: ObjectHolder<T>
}

impl<T> ObjectGuard<T> {
    pub fn new(obj: T, holder: ObjectHolder<T>) -> Self {
        ObjectGuard {
            obj: Some(obj),
            holder
        }
    }
}

impl<T> Drop for ObjectGuard<T> {
    fn drop(&mut self) {
        if let Some(obj) = self.obj.take() {
            self.holder.release(obj);
        }
    }
}

impl<T> Deref for ObjectGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.obj.as_ref().unwrap()
    }
}

impl<T> DerefMut for ObjectGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.obj.as_mut().unwrap()
    }
}

struct ObjectHolderState<T> {
    obj: Option<T>,
    waiter_list: Vec<NotifyFuture<T>>
}

pub struct ObjectHolder<T> {
    state: Arc<Mutex<ObjectHolderState<T>>>
}

impl<T> Clone for ObjectHolder<T> {
    fn clone(&self) -> Self {
        ObjectHolder {
            state: self.state.clone()
        }
    }
}
impl <T> ObjectHolder<T> {
    pub fn new(obj: T) -> Self {
        ObjectHolder {
            state: Arc::new(Mutex::new(ObjectHolderState {
                obj: Some(obj),
                waiter_list: vec![]
            }))
        }
    }

    pub async fn get(&self) -> ObjectGuard<T> {
        let waiter = {
            let mut state = self.state.lock().unwrap();
            if let Some(obj) = state.obj.take() {
                return ObjectGuard::new(obj, self.clone());
            }
            let waiter = NotifyFuture::new();
            state.waiter_list.push(waiter.clone());
            waiter
        };

        let obj = waiter.await;
        ObjectGuard::new(obj, self.clone())
    }

    fn release(&self, obj: T) {
        let mut state = self.state.lock().unwrap();
        if let Some(waiter) = state.waiter_list.pop() {
            waiter.set_complete(obj);
        } else {
            state.obj = Some(obj);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_object_holder() {
        let holder = ObjectHolder::new(1);
        let holder1 = holder.clone();
        let guard1 = tokio::spawn(async move {
            let guard = holder1.get().await;
            sleep(Duration::from_secs(1)).await;
        });

        let holder2 = holder.clone();
        let guard2 = tokio::spawn(async move {
            let guard = holder2.get().await;
        });

        guard1.await.unwrap();
        guard2.await.unwrap();
    }
}
