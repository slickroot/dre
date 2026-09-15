use pyo3::prelude::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::Write;

const CHUNK_SIZE: usize = 4096;
const DELETE_ALL: &str = "\x1b_Ga=d,d=A,q=2;\x1b\\";

fn zlib(pixels: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(pixels)
        .and_then(|_| encoder.finish())
        .expect("writes to an in-memory Vec<u8> can't fail")
}

fn base64(bytes: Vec<u8>) -> String {
    STANDARD.encode(bytes)
}

fn encode(pixels: &[u8]) -> String {
    base64(zlib(pixels))
}

fn chunks(payload: &str, chunk_size: usize) -> Vec<String> {
    payload
        .as_bytes()
        .chunks(chunk_size)
        .map(|chunk| {
            std::str::from_utf8(chunk)
                .expect("base64 payload is single-byte ASCII, so byte chunks are always valid UTF-8")
                .to_owned()
        })
        .collect()
}

fn more(chunks: &[String], index: usize) -> i32 {
    if index == chunks.len() - 1 { 0 } else { 1 }
}

fn escape(keys: &str, payload: &str) -> String {
    format!("\x1b_G{keys};{payload}\x1b\\")
}

fn transmission(pixels: &[u8], width: i64, height: i64) -> String {
    let payload = encode(pixels);
    let chunk_list = chunks(&payload, CHUNK_SIZE);
    let header = format!(
        "a=T,f=32,s={width},v={height},o=z,q=2,z=-1,m={}",
        more(&chunk_list, 0)
    );
    let mut escapes = vec![escape(&header, &chunk_list[0])];
    for (index, chunk) in chunk_list.iter().enumerate().skip(1) {
        let keys = format!("m={}", more(&chunk_list, index));
        escapes.push(escape(&keys, chunk));
    }
    escapes.join("")
}

#[pyclass]
struct KittyGraphics;

#[pymethods]
impl KittyGraphics {
    #[new]
    fn new() -> Self {
        KittyGraphics
    }

    fn draw(&self, sprites: Vec<Bound<'_, PyAny>>) -> PyResult<String> {
        let mut out = DELETE_ALL.to_string();
        for sprite in sprites {
            let row: i64 = sprite.getattr("row")?.extract()?;
            let col: i64 = sprite.getattr("col")?.extract()?;
            let width: i64 = sprite.getattr("width")?.extract()?;
            let height: i64 = sprite.getattr("height")?.extract()?;
            let pixels: Vec<u8> = sprite.getattr("pixels")?.extract()?;
            out.push_str(&format!("\x1b[{};{}H", row + 1, col + 1));
            out.push_str(&transmission(&pixels, width, height));
        }
        Ok(out)
    }
}

const PLAIN: i64 = -1;
const PALETTE_SIZE: i64 = 5;
const PAD: &str = " ";

#[pyclass(name = "Box", get_all, eq)]
#[derive(Clone, Debug, PartialEq)]
struct Node {
    label: String,
    colour: i64,
    fill: i64,
    rounded: bool,
    children: Vec<Node>,
}

impl Default for Node {
    fn default() -> Self {
        Node {
            label: String::new(),
            colour: PLAIN,
            fill: PLAIN,
            rounded: false,
            children: Vec::new(),
        }
    }
}

#[pymethods]
impl Node {
    #[new]
    #[pyo3(signature = (label="".to_string(), colour=PLAIN, fill=PLAIN, rounded=false, children=vec![]))]
    fn new(label: String, colour: i64, fill: i64, rounded: bool, children: Vec<Node>) -> Self {
        Node { label, colour, fill, rounded, children }
    }
}

#[pyclass]
#[derive(Clone)]
struct State {
    #[pyo3(get)]
    boxes: Vec<Node>,
    #[pyo3(get)]
    running: bool,
    #[pyo3(get)]
    mode: String,
    #[pyo3(get)]
    selected: Vec<i64>,
    before: Option<Box<State>>,
}

#[pymethods]
impl State {
    #[new]
    #[pyo3(signature = (boxes=vec![], running=true, mode="command".to_string(), selected=vec![], before=None))]
    fn new(boxes: Vec<Node>, running: bool, mode: String, selected: Vec<i64>, before: Option<State>) -> Self {
        State { boxes, running, mode, selected, before: before.map(Box::new) }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Command {
    Undo,
    NewBox,
    NewSibling,
    SelectParent,
    SelectChild,
    SelectNext,
    SelectPrevious,
    EditLabel,
    RenameLabel,
    CycleColour,
    CycleSiblingsColour,
    CycleFill,
    ToggleRounded,
    Quit,
}

impl TryFrom<&str> for Command {
    type Error = ();

    fn try_from(key: &str) -> Result<Self, Self::Error> {
        Ok(match key {
            "u" => Command::Undo,
            "b" => Command::NewBox,
            "s" => Command::NewSibling,
            "h" => Command::SelectParent,
            "l" => Command::SelectChild,
            "j" => Command::SelectNext,
            "k" => Command::SelectPrevious,
            "i" => Command::EditLabel,
            "I" => Command::RenameLabel,
            "c" => Command::CycleColour,
            "C" => Command::CycleSiblingsColour,
            "f" => Command::CycleFill,
            "r" => Command::ToggleRounded,
            "q" => Command::Quit,
            _ => return Err(()),
        })
    }
}

fn is_undoable(command: Command) -> bool {
    matches!(
        command,
        Command::NewBox
            | Command::CycleColour
            | Command::CycleFill
            | Command::ToggleRounded
            | Command::CycleSiblingsColour
            | Command::RenameLabel
    )
}

fn next_colour(colour: i64) -> i64 {
    (colour + 2).rem_euclid(PALETTE_SIZE + 1) - 1
}

fn at(boxes: &[Node], path: &[i64]) -> Node {
    let mut node = boxes[path[0] as usize].clone();
    for &index in &path[1..] {
        node = node.children[index as usize].clone();
    }
    node
}

fn rewrite(boxes: &[Node], path: &[i64], f: &dyn Fn(&Node) -> Node) -> Vec<Node> {
    let index = path[0] as usize;
    let rest = &path[1..];
    let mut result = boxes.to_vec();
    let node = &boxes[index];
    let new_node = if !rest.is_empty() {
        let mut n = node.clone();
        n.children = rewrite(&node.children, rest, f);
        n
    } else {
        f(node)
    };
    result[index] = new_node;
    result
}

fn colour_row(boxes: &[Node], path: &[i64]) -> Vec<Node> {
    let parent = &path[..path.len() - 1];
    let siblings: Vec<Node> = if !parent.is_empty() {
        at(boxes, parent).children
    } else {
        boxes.to_vec()
    };
    let first_colour = siblings[0].colour;
    let uniform = siblings.iter().all(|b| b.colour == first_colour);
    let new_colour = if uniform { next_colour(first_colour) } else { 0 };
    let mut boxes = boxes.to_vec();
    for i in 0..siblings.len() {
        let mut sibling_path = parent.to_vec();
        sibling_path.push(i as i64);
        boxes = rewrite(&boxes, &sibling_path, &|node: &Node| {
            let mut n = node.clone();
            n.colour = new_colour;
            n
        });
    }
    boxes
}

fn grow(boxes: &[Node], path: &[i64]) -> (Vec<Node>, Vec<i64>) {
    if path.is_empty() {
        let mut boxes = boxes.to_vec();
        let new_index = boxes.len() as i64;
        boxes.push(Node { label: PAD.to_string(), ..Default::default() });
        (boxes, vec![new_index])
    } else {
        let new_index = at(boxes, path).children.len() as i64;
        let grown = rewrite(boxes, path, &|node: &Node| {
            let mut n = node.clone();
            n.children.push(Node { label: PAD.to_string(), ..Default::default() });
            n
        });
        let mut new_path = path.to_vec();
        new_path.push(new_index);
        (grown, new_path)
    }
}

fn drop_last_chars(s: &str, n: usize) -> String {
    let len = s.chars().count();
    s.chars().take(len.saturating_sub(n)).collect()
}

fn enter_insert(state: &State, base_label: &str) -> State {
    let label = format!("{base_label}{PAD}");
    let boxes = rewrite(&state.boxes, &state.selected, &|node: &Node| {
        let mut n = node.clone();
        n.label = label.clone();
        n
    });
    let mut new_state = state.clone();
    new_state.boxes = boxes;
    new_state.mode = "insert".to_string();
    new_state
}

fn handle_command(state: &State, key: &str) -> State {
    let command = match Command::try_from(key) {
        Ok(command) => command,
        Err(()) => return state.clone(),
    };
    let mut state = state.clone();
    if is_undoable(command) {
        let mut snapshot = state.clone();
        snapshot.before = None;
        state.before = Some(Box::new(snapshot));
    }
    match command {
        Command::Undo => match state.before.take() {
            Some(before) => *before,
            None => state,
        },
        Command::NewBox => {
            let (boxes, selected) = grow(&state.boxes, &state.selected);
            state.boxes = boxes;
            state.mode = "insert".to_string();
            state.selected = selected;
            state
        }
        Command::NewSibling => {
            if state.selected.is_empty() {
                return state;
            }
            let parent = &state.selected[..state.selected.len() - 1];
            let (boxes, selected) = grow(&state.boxes, parent);
            state.boxes = boxes;
            state.mode = "insert".to_string();
            state.selected = selected;
            state
        }
        Command::SelectParent => {
            if state.selected.len() <= 1 {
                return state;
            }
            state.selected = state.selected[..state.selected.len() - 1].to_vec();
            state
        }
        Command::SelectChild => {
            if state.selected.is_empty() {
                return state;
            }
            if at(&state.boxes, &state.selected).children.is_empty() {
                return state;
            }
            let mut selected = state.selected.clone();
            selected.push(0);
            state.selected = selected;
            state
        }
        Command::SelectNext => {
            if state.selected.is_empty() {
                return state;
            }
            let parent = &state.selected[..state.selected.len() - 1];
            let index = *state.selected.last().unwrap();
            let siblings = if !parent.is_empty() {
                at(&state.boxes, parent).children
            } else {
                state.boxes.clone()
            };
            if index + 1 >= siblings.len() as i64 {
                return state;
            }
            let mut selected = parent.to_vec();
            selected.push(index + 1);
            state.selected = selected;
            state
        }
        Command::SelectPrevious => {
            if state.selected.is_empty() {
                return state;
            }
            let parent = &state.selected[..state.selected.len() - 1];
            let index = *state.selected.last().unwrap();
            if index == 0 {
                return state;
            }
            let mut selected = parent.to_vec();
            selected.push(index - 1);
            state.selected = selected;
            state
        }
        Command::EditLabel => {
            if state.selected.is_empty() {
                return state;
            }
            let label = at(&state.boxes, &state.selected).label;
            enter_insert(&state, &label)
        }
        Command::RenameLabel => {
            if state.selected.is_empty() {
                return state;
            }
            enter_insert(&state, "")
        }
        Command::CycleColour => {
            if state.selected.is_empty() {
                return state;
            }
            state.boxes = rewrite(&state.boxes, &state.selected, &|node: &Node| {
                let mut n = node.clone();
                n.colour = next_colour(n.colour);
                n
            });
            state
        }
        Command::CycleSiblingsColour => {
            if state.selected.len() <= 1 {
                return state;
            }
            state.boxes = colour_row(&state.boxes, &state.selected);
            state
        }
        Command::CycleFill => {
            if state.selected.is_empty() {
                return state;
            }
            state.boxes = rewrite(&state.boxes, &state.selected, &|node: &Node| {
                let mut n = node.clone();
                n.fill = next_colour(n.fill);
                n
            });
            state
        }
        Command::ToggleRounded => {
            if state.selected.is_empty() {
                return state;
            }
            state.boxes = rewrite(&state.boxes, &state.selected, &|node: &Node| {
                let mut n = node.clone();
                n.rounded = !n.rounded;
                n
            });
            state
        }
        Command::Quit => {
            state.running = false;
            state
        }
    }
}

fn handle_insert(state: &State, key: &str) -> State {
    let label = at(&state.boxes, &state.selected).label;
    let mut state = state.clone();
    if key == "\x1b" {
        let trimmed = drop_last_chars(&label, 1);
        state.boxes = rewrite(&state.boxes, &state.selected, &|node: &Node| {
            let mut n = node.clone();
            n.label = trimmed.clone();
            n
        });
        state.mode = "command".to_string();
        return state;
    }
    if key == "\x7f" {
        let new_label = format!("{}{PAD}", drop_last_chars(&label, 2));
        state.boxes = rewrite(&state.boxes, &state.selected, &|node: &Node| {
            let mut n = node.clone();
            n.label = new_label.clone();
            n
        });
        return state;
    }
    if key >= "\x20" && key <= "\x7e" {
        let new_label = format!("{}{key}{PAD}", drop_last_chars(&label, 1));
        state.boxes = rewrite(&state.boxes, &state.selected, &|node: &Node| {
            let mut n = node.clone();
            n.label = new_label.clone();
            n
        });
        return state;
    }
    state
}

#[pyfunction]
fn handle_key(state: &State, key: &str) -> State {
    if state.mode == "insert" {
        handle_insert(state, key)
    } else {
        handle_command(state, key)
    }
}

#[pymodule]
fn dre_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<KittyGraphics>()?;
    m.add_class::<Node>()?;
    m.add_class::<State>()?;
    m.add_function(wrap_pyfunction!(handle_key, m)?)?;
    m.add("CHUNK_SIZE", CHUNK_SIZE)?;
    m.add("DELETE_ALL", DELETE_ALL)?;
    m.add("PLAIN", PLAIN)?;
    m.add("PAD", PAD)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::ZlibDecoder;
    use std::io::Read as _;

    #[test]
    fn chunks_splits_at_boundaries() {
        let payload = "a".repeat(10);
        let result = chunks(&payload, 3);
        assert_eq!(result, vec!["aaa", "aaa", "aaa", "a"]);
    }

    #[test]
    fn chunks_exact_multiple_of_chunk_size() {
        let payload = "a".repeat(6);
        let result = chunks(&payload, 3);
        assert_eq!(result, vec!["aaa", "aaa"]);
    }

    #[test]
    fn chunks_single_chunk_when_smaller_than_size() {
        let payload = "abc";
        let result = chunks(payload, 100);
        assert_eq!(result, vec!["abc"]);
    }

    #[test]
    fn more_returns_zero_for_last_index() {
        let list = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(more(&list, 2), 0);
    }

    #[test]
    fn more_returns_one_for_non_last_index() {
        let list = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(more(&list, 0), 1);
        assert_eq!(more(&list, 1), 1);
    }

    #[test]
    fn more_single_element_list_is_last() {
        let list = vec!["a".to_string()];
        assert_eq!(more(&list, 0), 0);
    }

    #[test]
    fn escape_wraps_keys_and_payload() {
        let result = escape("a=T,f=32", "PAYLOAD");
        assert_eq!(result, "\x1b_Ga=T,f=32;PAYLOAD\x1b\\");
    }

    #[test]
    fn encode_round_trips_through_zlib_and_base64() {
        let pixels: Vec<u8> = (0..=255).collect();
        let encoded = encode(&pixels);
        let compressed = STANDARD.decode(&encoded).unwrap();
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed).unwrap();
        assert_eq!(decompressed, pixels);
    }

    #[test]
    fn transmission_single_chunk_has_expected_header_and_m0() {
        let pixels = vec![1u8, 2, 3, 4];
        let result = transmission(&pixels, 2, 1);

        let expected_payload = encode(&pixels);
        let expected_header = "a=T,f=32,s=2,v=1,o=z,q=2,z=-1,m=0".to_string();
        let expected = escape(&expected_header, &expected_payload);
        assert_eq!(result, expected);
        assert!(result.starts_with("\x1b_Ga=T,f=32,s=2,v=1,o=z,q=2,z=-1,m=0;"));
        assert!(result.ends_with("\x1b\\"));
    }

    #[test]
    fn transmission_multi_chunk_splits_and_sets_more_flag() {
        // Build pixel data large enough that the base64-encoded, zlib-compressed
        // payload spans more than one CHUNK_SIZE chunk.
        let mut state: u32 = 0x9E3779B9;
        let pixels: Vec<u8> = (0..200_000u32)
            .map(|_| {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                (state >> 16) as u8
            })
            .collect();
        let result = transmission(&pixels, 100, 100);

        let payload = encode(&pixels);
        let chunk_list = chunks(&payload, CHUNK_SIZE);
        assert!(chunk_list.len() > 1, "expected payload to span multiple chunks");

        let header = format!(
            "a=T,f=32,s=100,v=100,o=z,q=2,z=-1,m={}",
            more(&chunk_list, 0)
        );
        let mut expected = escape(&header, &chunk_list[0]);
        for (index, chunk) in chunk_list.iter().enumerate().skip(1) {
            let keys = format!("m={}", more(&chunk_list, index));
            expected.push_str(&escape(&keys, chunk));
        }

        assert_eq!(result, expected);
        // first chunk should be marked "more data coming" (m=1)
        assert!(result.contains(",m=1;"));
        // and the transmission should end with the final escape terminator
        assert!(result.ends_with("\x1b\\"));
        // there should be more than one escape sequence emitted
        assert!(result.matches("\x1b_G").count() > 1);
    }

    fn node(label: &str) -> Node {
        Node { label: label.to_string(), ..Default::default() }
    }

    fn node_with_children(label: &str, children: Vec<Node>) -> Node {
        Node { label: label.to_string(), children, ..Default::default() }
    }

    #[test]
    fn boxes_are_equal() {
        assert_eq!(Node::default(), Node::default());
    }

    #[test]
    fn boxes_default_to_the_plain_colour() {
        assert_eq!(Node::default().colour, PLAIN);
    }

    #[test]
    fn boxes_default_to_the_plain_fill() {
        assert_eq!(Node::default().fill, PLAIN);
    }

    #[test]
    fn boxes_default_to_an_empty_label() {
        assert_eq!(Node::default(), node(""));
    }

    #[test]
    fn boxes_with_different_labels_are_not_equal() {
        assert_ne!(node("a"), node("b"));
    }

    #[test]
    fn boxes_default_to_no_children() {
        assert_eq!(Node::default().children, Vec::<Node>::new());
    }

    #[test]
    fn boxes_with_different_children_are_not_equal() {
        assert_ne!(
            node_with_children("", vec![node("a")]),
            node_with_children("", vec![node("b")])
        );
    }

    #[test]
    fn new_box_starts_with_square_corners() {
        assert_eq!(node("a").rounded, false);
    }

    #[test]
    fn at_a_single_index_returns_the_top_level_box() {
        let boxes = vec![node("a"), node("b")];
        assert_eq!(at(&boxes, &[1]), node("b"));
    }

    #[test]
    fn at_a_longer_path_walks_into_children() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        assert_eq!(at(&boxes, &[0, 1]), node("d"));
    }

    #[test]
    fn at_a_deep_path_walks_multiple_levels() {
        let boxes = vec![node_with_children(
            "a",
            vec![node_with_children("b", vec![node("c")])],
        )];
        assert_eq!(at(&boxes, &[0, 0, 0]), node("c"));
    }

    #[test]
    fn rewrite_replaces_the_top_level_box() {
        let boxes = vec![node("a"), node("b")];
        let result = rewrite(&boxes, &[1], &|_| node("z"));
        assert_eq!(result, vec![node("a"), node("z")]);
    }

    #[test]
    fn rewrite_replaces_a_nested_box_and_rebuilds_the_spine() {
        let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
        let result = rewrite(&boxes, &[0, 1], &|_| node("z"));
        assert_eq!(
            result,
            vec![node_with_children("a", vec![node("c"), node("z")])]
        );
    }

    #[test]
    fn rewrite_does_not_mutate_the_given_boxes() {
        let boxes = vec![node("a")];
        rewrite(&boxes, &[0], &|_| node("z"));
        assert_eq!(boxes, vec![node("a")]);
    }

    #[test]
    fn grow_on_the_canvas_appends_a_top_level_box() {
        let (boxes, path) = grow(&[], &[]);
        assert_eq!(boxes, vec![node(PAD)]);
        assert_eq!(path, vec![0]);
    }

    #[test]
    fn grow_on_the_canvas_appends_after_existing_boxes() {
        let (boxes, path) = grow(&[node("a")], &[]);
        assert_eq!(boxes, vec![node("a"), node(PAD)]);
        assert_eq!(path, vec![1]);
    }

    #[test]
    fn grow_on_a_box_appends_a_child() {
        let (boxes, path) = grow(&[node("a")], &[0]);
        assert_eq!(boxes, vec![node_with_children("a", vec![node(PAD)])]);
        assert_eq!(path, vec![0, 0]);
    }

    #[test]
    fn grow_on_a_box_with_a_child_appends_a_second_child() {
        let boxes = vec![node_with_children("a", vec![node("c")])];
        let (boxes, path) = grow(&boxes, &[0]);
        assert_eq!(
            boxes,
            vec![node_with_children("a", vec![node("c"), node(PAD)])]
        );
        assert_eq!(path, vec![0, 1]);
    }

    #[test]
    fn grow_does_not_mutate_the_given_boxes() {
        let boxes = vec![node("a")];
        grow(&boxes, &[]);
        assert_eq!(boxes, vec![node("a")]);
    }

    #[test]
    fn next_colour_cycles_through_the_palette_and_back_to_plain() {
        let mut colour = PLAIN;
        for _ in 0..PALETTE_SIZE {
            colour = next_colour(colour);
        }
        assert_ne!(colour, PLAIN);
        colour = next_colour(colour);
        assert_eq!(colour, PLAIN);
    }

    #[test]
    fn colour_row_advances_uniformly_coloured_siblings() {
        let boxes = vec![node("a"), node("b")];
        let mut a = node("a");
        a.colour = next_colour(PLAIN);
        let mut b = node("b");
        b.colour = next_colour(PLAIN);
        assert_eq!(colour_row(&boxes, &[0]), vec![a, b]);
    }

    #[test]
    fn colour_row_sets_mixed_siblings_to_the_first_palette_colour() {
        let mut a = node("a");
        a.colour = 0;
        let mut b = node("b");
        b.colour = 1;
        let boxes = vec![a, b];
        let mut expected_a = node("a");
        expected_a.colour = 0;
        let mut expected_b = node("b");
        expected_b.colour = 0;
        assert_eq!(colour_row(&boxes, &[0]), vec![expected_a, expected_b]);
    }

    #[test]
    fn colour_row_does_not_mutate_the_given_boxes() {
        let boxes = vec![node("a"), node("b")];
        colour_row(&boxes, &[0]);
        assert_eq!(boxes, vec![node("a"), node("b")]);
    }

    fn new_state<'py>(
        py: Python<'py>,
        boxes: Vec<Node>,
        mode: &str,
        selected: Vec<i64>,
    ) -> Bound<'py, State> {
        Bound::new(
            py,
            State {
                boxes,
                running: true,
                mode: mode.to_string(),
                selected,
                before: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn state_starts_running() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![], "command", vec![]);
            assert!(state.borrow().running);
        });
    }

    #[test]
    fn state_starts_in_command_mode() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![], "command", vec![]);
            assert_eq!(state.borrow().mode, "command");
        });
    }

    #[test]
    fn b_on_an_empty_canvas_appends_a_box_enters_insert_and_selects_it() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![], "command", vec![]);
            let result: State = handle_key(&state.borrow(), "b");
            assert_eq!(result.boxes, vec![node(PAD)]);
            assert_eq!(result.mode, "insert");
            assert_eq!(result.selected, vec![0]);
            assert!(result.running);
        });
    }

    #[test]
    fn b_on_an_empty_canvas_does_not_mutate_the_given_state() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![], "command", vec![]);
            handle_key(&state.borrow(), "b");
            assert_eq!(state.borrow().boxes, Vec::<Node>::new());
        });
    }

    #[test]
    fn b_on_a_selected_box_appends_and_selects_a_child() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a")], "command", vec![0]);
            let result = handle_key(&state.borrow(), "b");
            assert_eq!(result.boxes, vec![node_with_children("a", vec![node(PAD)])]);
            assert_eq!(result.selected, vec![0, 0]);
            assert_eq!(result.mode, "insert");
        });
    }

    #[test]
    fn a_second_b_on_the_same_parent_places_a_second_child() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a")], "command", vec![0]);
            let state = Bound::new(py, handle_key(&state.borrow(), "b")).unwrap();
            let state = Bound::new(py, handle_key(&state.borrow(), "\x1b")).unwrap();
            let state = Bound::new(py, handle_key(&state.borrow(), "h")).unwrap();
            let result = handle_key(&state.borrow(), "b");
            assert_eq!(
                result.boxes,
                vec![node_with_children("a", vec![node(""), node(PAD)])]
            );
            assert_eq!(result.selected, vec![0, 1]);
        });
    }

    #[test]
    fn unknown_key_returns_the_state_unchanged() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a")], "command", vec![]);
            let result = handle_key(&state.borrow(), "x");
            assert_eq!(result.boxes, state.borrow().boxes);
            assert_eq!(result.selected, state.borrow().selected);
            assert_eq!(result.mode, state.borrow().mode);
            assert_eq!(result.running, state.borrow().running);
        });
    }

    #[test]
    fn q_stops_the_state_and_preserves_boxes_and_selection() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a")], "command", vec![0]);
            let result = handle_key(&state.borrow(), "q");
            assert!(!result.running);
            assert_eq!(result.boxes, vec![node("a")]);
            assert_eq!(result.selected, vec![0]);
            assert_eq!(result.mode, "command");
        });
    }

    #[test]
    fn insert_mode_is_dispatched_separately() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node(PAD)], "insert", vec![0]);
            let result = handle_key(&state.borrow(), "q");
            assert!(result.running);
        });
    }

    #[test]
    fn s_on_a_top_level_box_appends_a_sibling() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a")], "command", vec![0]);
            let result = handle_key(&state.borrow(), "s");
            assert_eq!(result.boxes, vec![node("a"), node(PAD)]);
            assert_eq!(result.selected, vec![1]);
            assert_eq!(result.mode, "insert");
        });
    }

    #[test]
    fn s_on_a_child_box_appends_a_sibling_to_the_parents_children() {
        Python::with_gil(|py| {
            let boxes = vec![node_with_children("a", vec![node("b")])];
            let state = new_state(py, boxes, "command", vec![0, 0]);
            let result = handle_key(&state.borrow(), "s");
            assert_eq!(
                result.boxes,
                vec![node_with_children("a", vec![node("b"), node(PAD)])]
            );
            assert_eq!(result.selected, vec![0, 1]);
        });
    }

    #[test]
    fn s_with_no_selection_returns_the_state_unchanged() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a")], "command", vec![]);
            let result = handle_key(&state.borrow(), "s");
            assert_eq!(result.boxes, vec![node("a")]);
            assert_eq!(result.selected, Vec::<i64>::new());
        });
    }

    #[test]
    fn h_selects_the_parent_and_is_a_no_op_at_the_top() {
        Python::with_gil(|py| {
            let boxes = vec![node_with_children("a", vec![node("c")])];
            let state = new_state(py, boxes, "command", vec![0, 0]);
            let result = handle_key(&state.borrow(), "h");
            assert_eq!(result.selected, vec![0]);

            let result = Bound::new(py, result).unwrap();
            let result = handle_key(&result.borrow(), "h");
            assert_eq!(result.selected, vec![0]);
        });
    }

    #[test]
    fn h_on_an_empty_canvas_returns_the_state_unchanged() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![], "command", vec![]);
            let result = handle_key(&state.borrow(), "h");
            assert_eq!(result.selected, Vec::<i64>::new());
        });
    }

    #[test]
    fn h_in_insert_mode_types_the_letter_h() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node(&format!("a{PAD}"))], "insert", vec![0]);
            let result = handle_key(&state.borrow(), "h");
            assert_eq!(result.boxes, vec![node(&format!("ah{PAD}"))]);
        });
    }

    #[test]
    fn l_selects_the_first_child_or_keeps_selection_with_none() {
        Python::with_gil(|py| {
            let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
            let state = new_state(py, boxes, "command", vec![0]);
            let result = handle_key(&state.borrow(), "l");
            assert_eq!(result.selected, vec![0, 0]);

            let state = new_state(py, vec![node("a")], "command", vec![0]);
            let result = handle_key(&state.borrow(), "l");
            assert_eq!(result.selected, vec![0]);
        });
    }

    #[test]
    fn j_and_k_move_between_siblings_with_bounds() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a"), node("b")], "command", vec![0]);
            let result = handle_key(&state.borrow(), "j");
            assert_eq!(result.selected, vec![1]);

            let state = new_state(py, vec![node("a"), node("b")], "command", vec![1]);
            let result = handle_key(&state.borrow(), "j");
            assert_eq!(result.selected, vec![1]);

            let state = new_state(py, vec![node("a"), node("b")], "command", vec![1]);
            let result = handle_key(&state.borrow(), "k");
            assert_eq!(result.selected, vec![0]);

            let state = new_state(py, vec![node("a"), node("b")], "command", vec![0]);
            let result = handle_key(&state.borrow(), "k");
            assert_eq!(result.selected, vec![0]);
        });
    }

    #[test]
    fn i_enters_insert_and_appends_pad_to_the_selected_boxs_label() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a"), node("b")], "command", vec![1]);
            let result = handle_key(&state.borrow(), "i");
            assert_eq!(result.mode, "insert");
            assert_eq!(result.selected, vec![1]);
            assert_eq!(result.boxes, vec![node("a"), node(&format!("b{PAD}"))]);
        });
    }

    #[test]
    fn i_on_an_empty_canvas_returns_the_state_unchanged() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![], "command", vec![]);
            let result = handle_key(&state.borrow(), "i");
            assert_eq!(result.mode, "command");
            assert_eq!(result.boxes, Vec::<Node>::new());
        });
    }

    #[test]
    fn capital_i_clears_the_selected_boxs_label() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a"), node("b")], "command", vec![1]);
            let result = handle_key(&state.borrow(), "I");
            assert_eq!(result.mode, "insert");
            assert_eq!(result.boxes, vec![node("a"), node(PAD)]);
        });
    }

    #[test]
    fn typing_appends_to_the_selected_box_label() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node(&format!("h{PAD}"))], "insert", vec![0]);
            let result = handle_key(&state.borrow(), "i");
            assert_eq!(result.boxes, vec![node(&format!("hi{PAD}"))]);
        });
    }

    #[test]
    fn space_and_tilde_are_printable() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node(&format!("a{PAD}"))], "insert", vec![0]);
            let result = handle_key(&state.borrow(), " ");
            assert_eq!(result.boxes, vec![node(&format!("a {PAD}"))]);

            let state = new_state(py, vec![node(PAD)], "insert", vec![0]);
            let result = handle_key(&state.borrow(), "~");
            assert_eq!(result.boxes, vec![node(&format!("~{PAD}"))]);
        });
    }

    #[test]
    fn backspace_drops_the_last_character_and_is_a_no_op_when_empty() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node(&format!("hi{PAD}"))], "insert", vec![0]);
            let result = handle_key(&state.borrow(), "\x7f");
            assert_eq!(result.boxes, vec![node(&format!("h{PAD}"))]);

            let state = new_state(py, vec![node(PAD)], "insert", vec![0]);
            let result = handle_key(&state.borrow(), "\x7f");
            assert_eq!(result.boxes, vec![node(PAD)]);
        });
    }

    #[test]
    fn esc_returns_to_command_mode_and_trims_pad() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node(&format!("hi{PAD}"))], "insert", vec![0]);
            let result = handle_key(&state.borrow(), "\x1b");
            assert_eq!(result.mode, "command");
            assert_eq!(result.boxes, vec![node("hi")]);
        });
    }

    #[test]
    fn control_and_non_ascii_characters_return_the_state_unchanged() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node(&format!("hi{PAD}"))], "insert", vec![0]);
            let result = handle_key(&state.borrow(), "\x01");
            assert_eq!(result.boxes, vec![node(&format!("hi{PAD}"))]);

            let result = handle_key(&state.borrow(), "é");
            assert_eq!(result.boxes, vec![node(&format!("hi{PAD}"))]);
        });
    }

    #[test]
    fn cycle_colour_and_fill_advance_independently_and_cycle_back_to_plain() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a")], "command", vec![0]);
            let result = handle_key(&state.borrow(), "c");
            let mut expected = node("a");
            expected.colour = next_colour(PLAIN);
            assert_eq!(result.boxes, vec![expected]);
            assert_eq!(result.boxes[0].label, "a");

            let state = new_state(py, vec![node("a")], "command", vec![0]);
            let result = handle_key(&state.borrow(), "f");
            let mut expected = node("a");
            expected.fill = next_colour(PLAIN);
            assert_eq!(result.boxes, vec![expected]);

            let mut state_boxed = vec![node("a")];
            let mut colour = PLAIN;
            for _ in 0..=PALETTE_SIZE {
                let s = new_state(py, state_boxed.clone(), "command", vec![0]);
                let result = handle_key(&s.borrow(), "c");
                state_boxed = result.boxes;
                colour = state_boxed[0].colour;
            }
            assert_eq!(colour, PLAIN);
        });
    }

    #[test]
    fn toggle_rounded_flips_and_flips_back() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a")], "command", vec![0]);
            let result = handle_key(&state.borrow(), "r");
            assert_eq!(result.boxes[0].rounded, true);
            let state = new_state(py, result.boxes.clone(), "command", vec![0]);
            let result = handle_key(&state.borrow(), "r");
            assert_eq!(result.boxes[0].rounded, false);
        });
    }

    #[test]
    fn capital_c_on_a_top_level_box_does_nothing() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a"), node("b")], "command", vec![0]);
            let result = handle_key(&state.borrow(), "C");
            assert_eq!(result.boxes, state.borrow().boxes);
            assert_eq!(result.selected, state.borrow().selected);
        });
    }

    #[test]
    fn capital_c_advances_uniformly_coloured_siblings() {
        Python::with_gil(|py| {
            let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
            let state = new_state(py, boxes, "command", vec![0, 1]);
            let result = handle_key(&state.borrow(), "C");
            let mut c = node("c");
            c.colour = next_colour(PLAIN);
            let mut d = node("d");
            d.colour = next_colour(PLAIN);
            assert_eq!(result.boxes, vec![node_with_children("a", vec![c, d])]);
        });
    }

    #[test]
    fn u_after_b_restores_boxes_and_selected() {
        Python::with_gil(|py| {
            let before = new_state(py, vec![], "command", vec![]);
            let after = handle_key(&before.borrow(), "b");
            let after = Bound::new(py, after).unwrap();
            let after_escape = handle_key(&after.borrow(), "\x1b");
            let after_escape = Bound::new(py, after_escape).unwrap();
            let undone = handle_key(&after_escape.borrow(), "u");
            assert_eq!(undone.boxes, before.borrow().boxes);
            assert_eq!(undone.selected, before.borrow().selected);
            assert_eq!(undone.mode, before.borrow().mode);
        });
    }

    #[test]
    fn u_after_c_restores_boxes() {
        Python::with_gil(|py| {
            let before = new_state(py, vec![node("a")], "command", vec![0]);
            let after = handle_key(&before.borrow(), "c");
            let after = Bound::new(py, after).unwrap();
            let undone = handle_key(&after.borrow(), "u");
            assert_eq!(undone.boxes, before.borrow().boxes);
        });
    }

    #[test]
    fn u_after_c_with_nothing_selected_is_a_no_op() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a")], "command", vec![]);
            let after = handle_key(&state.borrow(), "c");
            let after = Bound::new(py, after).unwrap();
            let undone = handle_key(&after.borrow(), "u");
            assert_eq!(undone.boxes, state.borrow().boxes);
            assert_eq!(undone.selected, state.borrow().selected);
        });
    }

    #[test]
    fn u_with_no_previous_action_leaves_state_unchanged() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![], "command", vec![]);
            let result = handle_key(&state.borrow(), "u");
            assert_eq!(result.boxes, Vec::<Node>::new());
            assert_eq!(result.selected, Vec::<i64>::new());
        });
    }

    #[test]
    fn u_twice_in_a_row_does_not_redo() {
        Python::with_gil(|py| {
            let state = new_state(py, vec![node("a")], "command", vec![0]);
            let after_command = handle_key(&state.borrow(), "c");
            let after_command = Bound::new(py, after_command).unwrap();
            let after_first_undo = handle_key(&after_command.borrow(), "u");
            let after_first_undo = Bound::new(py, after_first_undo).unwrap();
            let after_second_undo = handle_key(&after_first_undo.borrow(), "u");
            assert_eq!(after_second_undo.boxes, after_first_undo.borrow().boxes);
            assert_eq!(after_second_undo.selected, after_first_undo.borrow().selected);
        });
    }

    #[test]
    fn movement_keys_do_not_clobber_an_existing_undo_snapshot() {
        Python::with_gil(|py| {
            let boxes = vec![node_with_children("a", vec![node("c"), node("d")])];
            let before = new_state(py, boxes, "command", vec![0, 0]);
            let after_command = handle_key(&before.borrow(), "c");
            let after_command = Bound::new(py, after_command).unwrap();
            let navigated = handle_key(&after_command.borrow(), "j");
            let navigated = Bound::new(py, navigated).unwrap();
            let navigated = handle_key(&navigated.borrow(), "h");
            let navigated = Bound::new(py, navigated).unwrap();
            let navigated = handle_key(&navigated.borrow(), "l");
            let navigated = Bound::new(py, navigated).unwrap();
            let navigated = handle_key(&navigated.borrow(), "k");
            let navigated = Bound::new(py, navigated).unwrap();
            let undone = handle_key(&navigated.borrow(), "u");
            assert_eq!(undone.boxes, before.borrow().boxes);
            assert_eq!(undone.selected, before.borrow().selected);
        });
    }

    #[test]
    fn q_does_not_clobber_an_existing_undo_snapshot() {
        Python::with_gil(|py| {
            let before = new_state(py, vec![node("a")], "command", vec![0]);
            let after_command = handle_key(&before.borrow(), "c");
            let after_command = Bound::new(py, after_command).unwrap();
            let after_quit = handle_key(&after_command.borrow(), "q");
            let after_quit = Bound::new(py, after_quit).unwrap();
            let undone = handle_key(&after_quit.borrow(), "u");
            assert_eq!(undone.boxes, before.borrow().boxes);
            assert_eq!(undone.selected, before.borrow().selected);
        });
    }

    #[test]
    fn u_after_an_insert_session_undoes_the_b_that_started_it() {
        Python::with_gil(|py| {
            let before = new_state(py, vec![], "command", vec![]);
            let after_b = handle_key(&before.borrow(), "b");
            let after_b = Bound::new(py, after_b).unwrap();
            let after_typing = handle_key(&after_b.borrow(), "h");
            let after_typing = Bound::new(py, after_typing).unwrap();
            let after_typing = handle_key(&after_typing.borrow(), "i");
            let after_typing = Bound::new(py, after_typing).unwrap();
            let after_escape = handle_key(&after_typing.borrow(), "\x1b");
            let after_escape = Bound::new(py, after_escape).unwrap();
            let undone = handle_key(&after_escape.borrow(), "u");
            assert_eq!(undone.boxes, before.borrow().boxes);
            assert_eq!(undone.selected, before.borrow().selected);
        });
    }

    #[test]
    fn u_after_capital_i_restores_the_boxs_previous_label() {
        Python::with_gil(|py| {
            let before = new_state(py, vec![node("a")], "command", vec![0]);
            let after = handle_key(&before.borrow(), "I");
            let after = Bound::new(py, after).unwrap();
            let after_escape = handle_key(&after.borrow(), "\x1b");
            let after_escape = Bound::new(py, after_escape).unwrap();
            let undone = handle_key(&after_escape.borrow(), "u");
            assert_eq!(undone.boxes, before.borrow().boxes);
        });
    }
}
