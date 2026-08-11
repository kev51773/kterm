use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitDirection {
    Horizontal, // left / right split
    Vertical,   // top / bottom split
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LayoutNode {
    Pane {
        tab_id: String,
    },
    Split {
        id: String,
        direction: SplitDirection,
        ratio: f32,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

use std::sync::atomic::{AtomicU32, Ordering};
static SPLIT_COUNTER: AtomicU32 = AtomicU32::new(1);

impl LayoutNode {
    pub fn contains_tab(&self, target_id: &str) -> bool {

        match self {
            LayoutNode::Pane { tab_id } => tab_id == target_id,
            LayoutNode::Split { first, second, .. } => {
                first.contains_tab(target_id) || second.contains_tab(target_id)
            }
        }
    }

    pub fn collect_tabs(&self) -> Vec<String> {
        let mut tabs = Vec::new();
        self.collect_tabs_recursive(&mut tabs);
        tabs
    }

    fn collect_tabs_recursive(&self, acc: &mut Vec<String>) {
        match self {
            LayoutNode::Pane { tab_id } => acc.push(tab_id.clone()),
            LayoutNode::Split { first, second, .. } => {
                first.collect_tabs_recursive(acc);
                second.collect_tabs_recursive(acc);
            }
        }
    }

    pub fn split_at(
        &mut self,
        target_tab_id: &str,
        direction: SplitDirection,
        new_tab_id: &str,
        insert_first: bool,
    ) -> bool {
        match self {
            LayoutNode::Pane { tab_id } if tab_id == target_tab_id => {
                let existing_pane = Box::new(LayoutNode::Pane {
                    tab_id: tab_id.clone(),
                });
                let new_pane = Box::new(LayoutNode::Pane {
                    tab_id: new_tab_id.to_string(),
                });
                let split_num = SPLIT_COUNTER.fetch_add(1, Ordering::SeqCst);
                let (first, second) = if insert_first {
                    (new_pane, existing_pane)
                } else {
                    (existing_pane, new_pane)
                };
                *self = LayoutNode::Split {
                    id: format!("split-{}", split_num),
                    direction,
                    ratio: 0.5,
                    first,
                    second,
                };
                true
            }

            LayoutNode::Split { first, second, .. } => {
                first.split_at(target_tab_id, direction, new_tab_id, insert_first)
                    || second.split_at(target_tab_id, direction, new_tab_id, insert_first)
            }
            _ => false,
        }
    }


    pub fn remove_tab(&mut self, target_tab_id: &str) -> bool {
        if !self.contains_tab(target_tab_id) {
            return false;
        }

        match self {
            LayoutNode::Pane { tab_id } if tab_id == target_tab_id => false,
            LayoutNode::Split { first, second, .. } => {
                if first.contains_tab(target_tab_id) {
                    if matches!(**first, LayoutNode::Pane { ref tab_id } if tab_id == target_tab_id) {
                        *self = *second.clone();
                        return true;
                    }
                    return first.remove_tab(target_tab_id);
                }
                if second.contains_tab(target_tab_id) {
                    if matches!(**second, LayoutNode::Pane { ref tab_id } if tab_id == target_tab_id) {
                        *self = *first.clone();
                        return true;
                    }
                    return second.remove_tab(target_tab_id);
                }
                false
            }
            _ => false,
        }
    }


    pub fn unsplit_pane(&mut self, target_tab_id: &str) -> bool {
        self.remove_tab(target_tab_id)
    }

    pub fn update_ratio(&mut self, split_id: &str, new_ratio: f32) -> bool {
        match self {
            LayoutNode::Split { id, ratio, first, second, .. } => {
                if id == split_id {
                    *ratio = new_ratio.clamp(0.1, 0.9);
                    true
                } else {
                    first.update_ratio(split_id, new_ratio) || second.update_ratio(split_id, new_ratio)
                }
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── contains_tab ────────────────────────────────────────────────

    #[test]
    fn pane_contains_its_own_tab() {
        let node = LayoutNode::Pane { tab_id: "tab-1".into() };
        assert!(node.contains_tab("tab-1"));
        assert!(!node.contains_tab("tab-2"));
    }

    #[test]
    fn split_contains_both_children() {
        let root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
        };
        assert!(root.contains_tab("tab-1"));
        assert!(root.contains_tab("tab-2"));
        assert!(!root.contains_tab("tab-3"));
    }

    #[test]
    fn nested_split_contains_deep_tab() {
        let root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Split {
                id: "s2".into(),
                direction: SplitDirection::Vertical,
                ratio: 0.5,
                first: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
                second: Box::new(LayoutNode::Pane { tab_id: "tab-3".into() }),
            }),
        };
        assert!(root.contains_tab("tab-1"));
        assert!(root.contains_tab("tab-2"));
        assert!(root.contains_tab("tab-3"));
    }

    // ── collect_tabs ────────────────────────────────────────────────

    #[test]
    fn collect_tabs_pane() {
        let node = LayoutNode::Pane { tab_id: "tab-1".into() };
        assert_eq!(node.collect_tabs(), vec!["tab-1"]);
    }

    #[test]
    fn collect_tabs_split() {
        let root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
        };
        let tabs = root.collect_tabs();
        assert_eq!(tabs, vec!["tab-1", "tab-2"]);
    }

    #[test]
    fn collect_tabs_nested() {
        let root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Split {
                id: "s2".into(),
                direction: SplitDirection::Vertical,
                ratio: 0.5,
                first: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
                second: Box::new(LayoutNode::Pane { tab_id: "tab-3".into() }),
            }),
        };
        assert_eq!(root.collect_tabs(), vec!["tab-1", "tab-2", "tab-3"]);
    }

    // ── split_at ────────────────────────────────────────────────────

    #[test]
    fn split_pane_right() {
        let mut root = LayoutNode::Pane { tab_id: "tab-1".into() };
        let result = root.split_at("tab-1", SplitDirection::Horizontal, "tab-2", false);
        assert!(result);
        assert!(root.contains_tab("tab-1"));
        assert!(root.contains_tab("tab-2"));
        if let LayoutNode::Split { direction, ratio, first, second, .. } = &root {
            assert_eq!(*direction, SplitDirection::Horizontal);
            assert_eq!(*ratio, 0.5);
            assert!(first.contains_tab("tab-1"));
            assert!(second.contains_tab("tab-2"));
        } else {
            panic!("expected Split node");
        }
    }

    #[test]
    fn split_pane_left_insert_first() {
        let mut root = LayoutNode::Pane { tab_id: "tab-1".into() };
        root.split_at("tab-1", SplitDirection::Horizontal, "tab-2", true);
        if let LayoutNode::Split { first, second, .. } = &root {
            assert!(first.contains_tab("tab-2"));
            assert!(second.contains_tab("tab-1"));
        } else {
            panic!("expected Split");
        }
    }

    #[test]
    fn split_pane_down() {
        let mut root = LayoutNode::Pane { tab_id: "tab-1".into() };
        root.split_at("tab-1", SplitDirection::Vertical, "tab-2", false);
        assert!(root.contains_tab("tab-1"));
        assert!(root.contains_tab("tab-2"));
    }

    #[test]
    fn split_nonexistent_tab_returns_false() {
        let mut root = LayoutNode::Pane { tab_id: "tab-1".into() };
        let result = root.split_at("tab-999", SplitDirection::Horizontal, "tab-2", false);
        assert!(!result);
    }

    #[test]
    fn split_deep_tab() {
        let mut root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
        };
        let result = root.split_at("tab-2", SplitDirection::Vertical, "tab-3", false);
        assert!(result);
        assert!(root.contains_tab("tab-3"));
    }

    // ── remove_tab ──────────────────────────────────────────────────

    #[test]
    fn remove_tab_from_split_promotes_sibling() {
        let mut root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
        };
        let result = root.remove_tab("tab-1");
        assert!(result);
        // After removing tab-1, root should be the Pane for tab-2
        assert!(root.contains_tab("tab-2"));
        assert!(!root.contains_tab("tab-1"));
    }

    #[test]
    fn remove_tab_not_found() {
        let mut root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
        };
        let result = root.remove_tab("tab-999");
        assert!(!result);
        assert!(root.contains_tab("tab-1"));
        assert!(root.contains_tab("tab-2"));
    }

    #[test]
    fn remove_second_tab_promotes_first() {
        let mut root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
        };
        root.remove_tab("tab-2");
        assert!(root.contains_tab("tab-1"));
        assert!(!root.contains_tab("tab-2"));
    }

    // ── unsplit_pane ────────────────────────────────────────────────

    #[test]
    fn unsplit_pane_delegates_to_remove() {
        let mut root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
        };
        let result = root.unsplit_pane("tab-1");
        assert!(result);
        assert!(!root.contains_tab("tab-1"));
    }

    // ── update_ratio ────────────────────────────────────────────────

    #[test]
    fn update_ratio_direct() {
        let mut root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
        };
        let result = root.update_ratio("s1", 0.7);
        assert!(result);
        if let LayoutNode::Split { ratio, .. } = &root {
            assert_eq!(*ratio, 0.7);
        }
    }

    #[test]
    fn update_ratio_clamped_low() {
        let mut root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
        };
        root.update_ratio("s1", 0.01);
        if let LayoutNode::Split { ratio, .. } = &root {
            assert_eq!(*ratio, 0.1);
        }
    }

    #[test]
    fn update_ratio_clamped_high() {
        let mut root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
        };
        root.update_ratio("s1", 0.99);
        if let LayoutNode::Split { ratio, .. } = &root {
            assert_eq!(*ratio, 0.9);
        }
    }

    #[test]
    fn update_ratio_nested() {
        let mut root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Split {
                id: "s2".into(),
                direction: SplitDirection::Vertical,
                ratio: 0.5,
                first: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
                second: Box::new(LayoutNode::Pane { tab_id: "tab-3".into() }),
            }),
        };
        let result = root.update_ratio("s2", 0.3);
        assert!(result);
    }

    #[test]
    fn update_ratio_not_found() {
        let mut root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
        };
        let result = root.update_ratio("s999", 0.7);
        assert!(!result);
    }

    // ── split_at then remove roundtrip ──────────────────────────────

    #[test]
    fn split_then_remove_restores() {
        let mut root = LayoutNode::Pane { tab_id: "tab-1".into() };
        root.split_at("tab-1", SplitDirection::Horizontal, "tab-2", false);
        assert!(root.contains_tab("tab-2"));
        root.remove_tab("tab-2");
        // Should be back to just tab-1
        assert!(!root.contains_tab("tab-2"));
        assert!(root.contains_tab("tab-1"));
    }

    // ── serialization roundtrip ─────────────────────────────────────

    #[test]
    fn layout_serde_roundtrip() {
        let root = LayoutNode::Split {
            id: "s1".into(),
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(LayoutNode::Pane { tab_id: "tab-1".into() }),
            second: Box::new(LayoutNode::Pane { tab_id: "tab-2".into() }),
        };
        let json = serde_json::to_string(&root).unwrap();
        let deserialized: LayoutNode = serde_json::from_str(&json).unwrap();
        assert!(deserialized.contains_tab("tab-1"));
        assert!(deserialized.contains_tab("tab-2"));
    }
}
