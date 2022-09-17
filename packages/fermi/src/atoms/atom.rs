use crate::{AtomId, AtomRoot, Readable, Writable};

pub type Atom<T> = fn(AtomBuilder) -> T;
pub struct AtomBuilder;

impl<V: 'static> Readable<V> for Atom<V> {
    fn read(&self, _root: AtomRoot) -> Option<V> {
        todo!()
    }
    fn init(&self) -> V {
        (*self)(AtomBuilder)
    }
    fn unique_id(&self) -> AtomId {
        AtomId {
            ptr: *self as *const (),
            type_id: std::any::TypeId::of::<V>(),
        }
    }
}

impl<V: 'static> Writable<V> for Atom<V> {
    fn write(&self, _root: AtomRoot, _value: V) {
        todo!()
    }
}

#[test]
fn atom_compiles() {
    static TEST_ATOM: Atom<&str> = |_| "hello";
    dbg!(TEST_ATOM.init());
}
