use std::collections::HashMap;
use std::hash::Hash;

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use slotmap::basic::Drain;
use slotmap::{Key, SlotMap};

pub type ScopeIds<Key> = IndexSet<Key>;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct ScopedSlotMap<SlotMapKey, SlotMapData, Scope>
where
    SlotMapKey: Key,
    Scope: Clone + Hash + Eq,
{
    slotmap: SlotMap<SlotMapKey, (Scope, SlotMapData)>,
    scope_elements: HashMap<Scope, ScopeIds<SlotMapKey>>,
}

#[allow(dead_code)]
impl<SlotMapKey, Scope, SlotMapData> ScopedSlotMap<SlotMapKey, SlotMapData, Scope>
where
    SlotMapKey: Key,
    Scope: Clone + Hash + Eq,
{
    pub(crate) fn get(&self, key: SlotMapKey) -> Option<&SlotMapData> {
        if let Some((_scope, data)) = self.slotmap.get(key) {
            Some(data)
        } else {
            None
        }
    }

    pub(crate) fn get_mut(&mut self, key: SlotMapKey) -> Option<&mut SlotMapData> {
        if let Some((_scope, data)) = self.slotmap.get_mut(key) {
            Some(data)
        } else {
            None
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (SlotMapKey, &(Scope, SlotMapData))> + '_ {
        self.slotmap.iter()
    }

    pub(crate) fn get_scope(&self, key: SlotMapKey) -> Option<Scope> {
        if let Some((scope, _data)) = self.slotmap.get(key) {
            Some(scope.clone())
        } else {
            None
        }
    }

    pub(crate) fn scope(&self, sm_scope: Scope) -> ScopeIds<SlotMapKey> {
        if let Some(elements) = self.scope_elements.get(&sm_scope) {
            elements.to_owned()
        } else {
            Default::default()
        }
    }

    pub(crate) fn elements_in_scope(&self, sm_scope: Scope) -> usize {
        if let Some(elements) = self.scope_elements.get(&sm_scope) {
            elements.len()
        } else {
            0
        }
    }

    pub(crate) fn reserve(&mut self, additional: usize) {
        self.slotmap.reserve(additional);
        self.scope_elements.reserve(additional);
    }

    pub(crate) fn insert(&mut self, idx: Scope, data: SlotMapData) -> SlotMapKey {
        let slotmap_key = self.slotmap.insert((idx.clone(), data));
        if let Some(indices) = self.scope_elements.get_mut(&idx) {
            indices.insert(slotmap_key);
        } else {
            self.scope_elements
                .insert(idx, ScopeIds::<SlotMapKey>::from([slotmap_key]));
        }
        slotmap_key
    }

    pub(crate) fn contains_key(&self, key: SlotMapKey) -> bool {
        self.slotmap.contains_key(key)
    }

    pub(crate) fn remove(&mut self, key: SlotMapKey) -> Option<(SlotMapData, Scope)> {
        let rm_element = self.slotmap.remove(key);
        if let Some((idx, data)) = rm_element {
            if let Some(indices) = self.scope_elements.get_mut(&idx) {
                indices.swap_remove(&key);
                Some((data, idx))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub(crate) fn clear(&mut self) {
        self.slotmap.clear();
        self.scope_elements.clear();
    }

    pub(crate) fn drain(&mut self) -> Drain<'_, SlotMapKey, (Scope, SlotMapData)> {
        self.scope_elements.drain();
        self.slotmap.drain()
    }

    pub(crate) fn len(&self) -> usize {
        self.slotmap.len()
    }

    pub(crate) fn capacity(&self) -> usize {
        self.slotmap.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::*;

    new_key_type! {
        struct ScopedMapExampleKey;
    }

    #[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
    enum ExampleScope {
        #[default]
        Top,
        Definition(u64),
        Expression(u64, u32),
    }

    #[test]
    fn insert_and_remove_scoped_map() {
        let mut scoped_sm: ScopedSlotMap<ScopedMapExampleKey, String, ExampleScope> =
            ScopedSlotMap::default();
        let _first_element = scoped_sm.insert(ExampleScope::Top, "TOP".to_string());
        let _second_element = scoped_sm.insert(ExampleScope::Definition(0), "Def1".to_string());
        let third_element = scoped_sm.insert(ExampleScope::Expression(0, 0), "Expr1".to_string());
        assert_eq!(3, scoped_sm.len(), "There should be 3 elements in the map.");
        assert_eq!(
            3,
            scoped_sm.capacity(),
            "There should be 3 elements reserved in the map."
        );
        scoped_sm.remove(third_element);
        assert_eq!(2, scoped_sm.len(), "There should be 2 elements in the map.");
        assert_eq!(
            3,
            scoped_sm.capacity(),
            "There should be 3 elements reserved in the map."
        );
    }

    #[test]
    fn lookup_elements_in_scope() {
        let mut scoped_sm: ScopedSlotMap<ScopedMapExampleKey, String, ExampleScope> =
            ScopedSlotMap::default();
        let first_element = scoped_sm.insert(ExampleScope::Top, "TOP".to_string());
        let second_element = scoped_sm.insert(ExampleScope::Definition(0), "Def1".to_string());
        let third_element = scoped_sm.insert(ExampleScope::Expression(0, 0), "Expr1".to_string());
        let fourth_element = scoped_sm.insert(ExampleScope::Expression(0, 0), "Expr2".to_string());
        assert_eq!(1, scoped_sm.elements_in_scope(ExampleScope::Top));
        let top_elements = scoped_sm.scope(ExampleScope::Top);
        assert_eq!(first_element, *top_elements.iter().next().unwrap());
        assert_eq!(1, scoped_sm.elements_in_scope(ExampleScope::Definition(0)));
        let def_elements = scoped_sm.scope(ExampleScope::Definition(0));
        assert_eq!(second_element, *def_elements.iter().next().unwrap());
        assert_eq!(
            2,
            scoped_sm.elements_in_scope(ExampleScope::Expression(0, 0))
        );
        let exp_elements = scoped_sm.scope(ExampleScope::Expression(0, 0)).to_owned();
        assert_eq!(third_element, *exp_elements.iter().next().unwrap());
        assert_eq!(fourth_element, *exp_elements.iter().nth(1).unwrap());
        scoped_sm.remove(third_element);
        let exp_elements_post_removal = scoped_sm.scope(ExampleScope::Expression(0, 0)).to_owned();
        assert_eq!(
            fourth_element,
            *exp_elements_post_removal.iter().next().unwrap()
        );
        assert!(exp_elements_post_removal.iter().nth(1).is_none());
    }

    #[test]
    fn associate_data_with_secondary_map() {
        let mut scoped_sm: ScopedSlotMap<ScopedMapExampleKey, String, ExampleScope> =
            ScopedSlotMap::default();
        let mut scoped_secondary_map: SecondaryMap<ScopedMapExampleKey, String> =
            Default::default();
        let _first_element = scoped_sm.insert(ExampleScope::Top, "TOP".to_string());
        let _second_element = scoped_sm.insert(ExampleScope::Definition(0), "Def1".to_string());
        let third_element = scoped_sm.insert(ExampleScope::Expression(0, 0), "Expr1".to_string());
        let expr_name: String = "x".to_owned();
        scoped_secondary_map.insert(third_element, expr_name);
        assert_eq!("x", scoped_secondary_map[third_element]);
    }

    #[test]
    fn test_empty_map_operations() {
        let empty_map: ScopedSlotMap<ScopedMapExampleKey, String, ExampleScope> =
            ScopedSlotMap::default();
        assert_eq!(empty_map.len(), 0);
        assert_eq!(empty_map.capacity(), 0);

        // Test operations on empty map
        let fake_key = ScopedMapExampleKey::default();
        assert!(empty_map.get(fake_key).is_none());
        assert!(!empty_map.contains_key(fake_key));
        assert!(empty_map.get_scope(fake_key).is_none());
    }

    #[test]
    fn test_remove_nonexistent_key() {
        let mut map: ScopedSlotMap<ScopedMapExampleKey, String, ExampleScope> =
            ScopedSlotMap::default();
        let key = ScopedMapExampleKey::default();
        assert!(map.remove(key).is_none());
    }

    #[test]
    fn test_clear_empty_map() {
        let mut map: ScopedSlotMap<ScopedMapExampleKey, String, ExampleScope> =
            ScopedSlotMap::default();
        map.clear();
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_multiple_inserts_same_scope() {
        let mut map: ScopedSlotMap<ScopedMapExampleKey, String, ExampleScope> =
            ScopedSlotMap::default();
        let scope = ExampleScope::Top;

        // Insert multiple items in same scope
        let key1 = map.insert(scope.clone(), "first".to_string());
        let key2 = map.insert(scope.clone(), "second".to_string());
        let key3 = map.insert(scope.clone(), "third".to_string());

        assert_eq!(map.elements_in_scope(scope.clone()), 3);
        assert_eq!(map.get(key1), Some(&"first".to_string()));
        assert_eq!(map.get(key2), Some(&"second".to_string()));
        assert_eq!(map.get(key3), Some(&"third".to_string()));
    }

    #[test]
    fn test_remove_and_reinsert() {
        let mut map: ScopedSlotMap<ScopedMapExampleKey, String, ExampleScope> =
            ScopedSlotMap::default();
        let scope = ExampleScope::Top;

        // Insert and remove
        let key = map.insert(scope.clone(), "test".to_string());
        assert_eq!(map.elements_in_scope(scope.clone()), 1);

        let (data, removed_scope) = map.remove(key).unwrap();
        assert_eq!(data, "test".to_string());
        assert_eq!(removed_scope, scope);
        assert_eq!(map.elements_in_scope(scope.clone()), 0);

        // Reinsert with same scope
        let new_key = map.insert(scope.clone(), data);
        assert_eq!(map.elements_in_scope(scope.clone()), 1);
        assert!(map.contains_key(new_key));
    }

    #[test]
    fn test_scope_isolation() {
        let mut map: ScopedSlotMap<ScopedMapExampleKey, String, ExampleScope> =
            ScopedSlotMap::default();

        // Insert into different scopes
        let scope1 = ExampleScope::Top;
        let scope2 = ExampleScope::Definition(1);

        let key1 = map.insert(scope1.clone(), "scope1".to_string());
        let key2 = map.insert(scope2.clone(), "scope2".to_string());

        assert_eq!(map.elements_in_scope(scope1.clone()), 1);
        assert_eq!(map.elements_in_scope(scope2.clone()), 1);
        assert_eq!(map.get_scope(key1), Some(scope1.clone()));
        assert_eq!(map.get_scope(key2), Some(scope2.clone()));
    }

    #[test]
    fn test_mutation_through_get_mut() {
        let mut map: ScopedSlotMap<ScopedMapExampleKey, String, ExampleScope> =
            ScopedSlotMap::default();
        let scope = ExampleScope::Top;

        let key = map.insert(scope.clone(), "original".to_string());

        // Modify through get_mut
        if let Some(value) = map.get_mut(key) {
            *value = "modified".to_string();
        }

        assert_eq!(map.get(key), Some(&"modified".to_string()));
    }

    #[test]
    fn test_reserve_and_capacity() {
        let mut map: ScopedSlotMap<ScopedMapExampleKey, String, ExampleScope> =
            ScopedSlotMap::default();

        map.reserve(10);
        assert!(map.capacity() >= 10);

        // Insert fewer elements than reserved
        for i in 0..5 {
            map.insert(ExampleScope::Definition(i), i.to_string());
        }

        assert_eq!(map.len(), 5);
        assert!(map.capacity() >= 10);
    }

    #[test]
    fn test_drain_operations() {
        let mut map: ScopedSlotMap<ScopedMapExampleKey, String, ExampleScope> =
            ScopedSlotMap::default();

        // Insert some elements
        let keys: Vec<_> = (0..3)
            .map(|i| map.insert(ExampleScope::Definition(i as u64), format!("value{}", i)))
            .collect();

        assert_eq!(map.len(), 3);

        // Drain and collect
        let drained: Vec<_> = map.drain().collect();
        assert_eq!(drained.len(), 3);
        assert_eq!(map.len(), 0);

        // Verify all scopes are cleared
        for key in keys {
            assert!(!map.contains_key(key));
        }
    }
}
