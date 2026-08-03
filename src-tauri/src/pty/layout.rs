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
            LayoutNode::Pane { tab_id } if tab_id == target_tab_id => {
                false
            }
            LayoutNode::Split { first, second, .. } => {
                if matches!(**first, LayoutNode::Pane { ref tab_id } if tab_id == target_tab_id) {
                    *self = *second.clone();
                    return true;
                }
                if matches!(**second, LayoutNode::Pane { ref tab_id } if tab_id == target_tab_id) {
                    *self = *first.clone();
                    return true;
                }

                first.remove_tab(target_tab_id) || second.remove_tab(target_tab_id)
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
