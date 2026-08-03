# Phase 3 Implementation Plan - Target-Based Splits & UI Layout

## Objective
Implement target-based incremental split pane management (`--split-right`, `--split-down`, `--move-tab`, `--unsplit`, `--explode-split`), construct the frontend split pane flex renderer, add right-click context menus, and register keyboard shortcuts.

---

## 1. Incremental Split Data Model & API

### API Route Endpoint: `POST /tabs/:id/split`
```json
// Request Body
{
  "direction": "right",          // "right" | "down" | "left" | "up"
  "profile": "git-bash",         // optional: spawn new shell in split
  "move_tab_id": "tab-104"       // optional: move existing standalone tab into split
}
```

### Response
```json
{
  "split_id": "split-1",
  "new_tab_id": "tab-102",
  "target_tab_id": "tab-101"
}
```

---

## 2. Layout Tree Data Structure (`src-tauri/src/pty/layout.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LayoutNode {
    Pane {
        tab_id: String,
    },
    Split {
        id: String,
        direction: SplitDirection, // Horizontal | Vertical
        ratio: f32,                // Default 0.5
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

impl LayoutNode {
    pub fn split_at(&mut self, target_tab_id: &str, direction: SplitDirection, new_tab_id: &str) -> bool {
        match self {
            LayoutNode::Pane { tab_id } if tab_id == target_tab_id => {
                let existing_pane = Box::new(LayoutNode::Pane { tab_id: tab_id.clone() });
                let new_pane = Box::new(LayoutNode::Pane { tab_id: new_tab_id.to_string() });
                *self = LayoutNode::Split {
                    id: format!("split-{}", uuid::Uuid::new_v4()),
                    direction,
                    ratio: 0.5,
                    first: existing_pane,
                    second: new_pane,
                };
                true
            }
            LayoutNode::Split { first, second, .. } => {
                first.split_at(target_tab_id, direction, new_tab_id) || second.split_at(target_tab_id, direction, new_tab_id)
            }
            _ => false,
        }
    }
}
```

---

## 3. Frontend Split Pane Renderer (`src/components/SplitGrid.ts`)

- Recursively traverses `LayoutNode` JSON returned by backend WS/REST update events.
- Renders nested flex containers (`display: flex; flex-direction: row | column`).
- Resizable split bars with drag handle (`ratio` adjustment).
- Each pane container mounts an `xterm.js` terminal instance bound to the corresponding PTY WebSocket stream (`ws://127.0.0.1:9999/tabs/:id/ws`).

---

## 4. CLI Split Commands Spec

```powershell
# Split off $t1 to the right (returns $t2)
$t2 = .\kterm.exe --select-tab $t1 --split-right --profile git-bash

# Split off $t1 to the left
.\kterm.exe --select-tab $t1 --split-left --profile git-bash

# Split off $t2 downward (returns $t3)
$t3 = .\kterm.exe --select-tab $t2 --split-down --profile wsl

# Split off $t2 upward
.\kterm.exe --select-tab $t2 --split-up --profile wsl

# Move existing tab $t4 into space below $t1
.\kterm.exe --select-tab $t1 --split-down --move-tab $t4

# Un-split single pane back into standalone tab
.\kterm.exe --select-tab $t3 --unsplit

# Separate all panes in split into top-level tabs
.\kterm.exe --select-tab $t1 --explode-split
```

---

## 5. UI Context Menu & Keyboard Shortcuts

- **ContextMenu**: Right-click pane -> "Split Right (`Ctrl+Shift+Right`)", "Split Left (`Ctrl+Shift+Left`)", "Split Down (`Ctrl+Shift+Down`)", "Split Up (`Ctrl+Shift+Up`)", "Un-split Pane (`Ctrl+Shift+Del`)", "Close Pane".
- **Keybindings**:
  - `Ctrl+Shift+Right`: Split Right
  - `Ctrl+Shift+Left`: Split Left
  - `Ctrl+Shift+Down`: Split Down
  - `Ctrl+Shift+Up`: Split Up
  - `Ctrl+Shift+Del`: Un-split active pane / close split
- **Behavioral Polish**:
  - **Profile Inheritance**: Splitting a pane without specifying `--profile` inherits the profile of the target pane.
  - **Graceful Exit**: `exit 0` clean process exit automatically closes pane and collapses layout. Non-zero exit code shows warning notice before closing.
  - **Sibling Focus Inheritance**: When a split pane closes, focus transitions to a sibling pane in the same split group.

