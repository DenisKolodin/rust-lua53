extern crate lua;

use std::sync::{Arc, Weak, Mutex};

#[derive(PartialEq)]
struct ExtraData {
    value: String,
}

impl ExtraData {
    fn new_arc(val: &str) -> Arc<lua::Extra> {
        Arc::new(Box::new(Mutex::new(ExtraData {
            value: val.to_owned(),
        })))
    }
}

fn with_extra<F, R>(state: &mut lua::State, f: F) -> R
    where F: FnOnce(&mut ExtraData) -> R {
    let arc = state.get_extra().upgrade().expect("arc to data");
    let mutex = arc.downcast_ref::<Mutex<ExtraData>>().expect("donwcast extra reference");
    let mut extra = mutex.lock().unwrap();
    f(&mut *extra)
}

fn unwrap_extra(state: &mut lua::State, own: Arc<lua::Extra>) -> ExtraData {
    let arc = state.set_extra(Weak::new()).upgrade().expect("replace arc");
    drop(own);
    let boxed_extra = Arc::try_unwrap(arc).expect("unwrap arc");
    let mutex = boxed_extra.downcast::<Mutex<ExtraData>>().expect("downcast extra data");
    let extra = mutex.into_inner().unwrap();
    extra
}

#[test]
fn test_extra_owned() {
    let mut state = lua::State::new();

    assert!(state.get_extra().upgrade().is_none());
    assert!(state.set_extra(Weak::new()).upgrade().is_none());

    let extra = ExtraData::new_arc("Initial data");
    state.set_extra(Arc::downgrade(&extra));

    for x in 0..10 {
        with_extra(&mut state, |extra| {
            extra.value = format!("Changed to {}", x);
        });
    }

    assert_eq!(unwrap_extra(&mut state, extra).value, "Changed to 9");

    assert!(state.get_extra().upgrade().is_none());
    assert!(state.set_extra(Weak::new()).upgrade().is_none());
}

#[test]
fn test_extra_thread() {
    let mut state = lua::State::new();

    let mut thread = state.new_thread();
    assert!(thread.get_extra().upgrade().is_none());
    assert!(thread.set_extra(Weak::new()).upgrade().is_none());
    assert!(state.get_extra().upgrade().is_none());
    assert!(state.set_extra(Weak::new()).upgrade().is_none());

    let extra = ExtraData::new_arc("Be shared!");
    state.set_extra(Arc::downgrade(&extra));
    with_extra(&mut state, |extra| {
        assert_eq!(extra.value, "Be shared!");
    });

    let mut thread = state.new_thread();
    with_extra(&mut thread, |extra| {
        assert_eq!(extra.value, "Be shared!");
    });

    let local_extra = ExtraData::new_arc("I'm in thread!");
    thread.set_extra(Arc::downgrade(&local_extra));

    with_extra(&mut thread, |extra| {
        assert_eq!(extra.value, "I'm in thread!");
    });

    with_extra(&mut state, |extra| {
        assert_eq!(extra.value, "Be shared!");
    });

    assert!(thread.get_extra().upgrade().is_some());
    drop(local_extra);
    assert!(thread.get_extra().upgrade().is_none());

    with_extra(&mut state, |extra| {
        assert_eq!(extra.value, "Be shared!");
    });

}
