#![allow(dead_code, unused_imports)]
use gloo_console as console;
use js_sys::Date;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew::{html, Component, Context, Html};

pub enum Msg {
    AddTx,
    HighlightTx(usize),
}

type ShortHash = String;
#[derive(Hash, Debug, Clone)]
struct Tx {
    from: String,
    to: String,
    amount: u64,
    id: usize,
}

impl Tx {
    fn new() -> Self {
        Tx {
            from: "".to_string(),
            to: "".to_string(),
            amount: 0,
            id: 0,
        }
    }
    fn new_hash(&self) -> String {
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        base62::encode(s.finish())
    }
}

pub struct NewTx {
    from: NodeRef,
    to: NodeRef,
    amount: NodeRef,
}

impl NewTx {
    fn new() -> Self {
        NewTx {
            to: NodeRef::default(),
            from: NodeRef::default(),
            amount: NodeRef::default(),
        }
    }
}

fn get_color(height: usize) -> String {
    let colors = vec![
        "background-color: #000b0b;",
        "background-color: #000a0a;",
        "background-color: #000909;",
        "background-color: #000808;",
        "background-color: #000008;",
        "background-color: #000800;",
        "background-color: #000707;",
        "background-color: #000007;",
        "background-color: #000700;",
        "background-color: #000600;",
        "background-color: #000006;",
        "background-color: #000500;",
        "background-color: #000005;",
        "background-color: #000004;",
        "background-color: #000400;",
    ];
    colors[height % 15].to_string()
}

#[derive(Debug)]
struct Node {
    down: Option<Box<Node>>,
    up: Option<Box<Node>>,
    hash: ShortHash,
    tx: Option<Tx>,
    highlight: bool,
}

impl Node {
    fn new() -> Node {
        Node {
            down: None,
            up: None,
            hash: String::new(),
            tx: None,
            highlight: false,
        }
    }

    fn bootstrap(tx: Tx) -> Node {
        let hash = tx.new_hash();
        Node {
            down: Some(Box::new(Node {
                down: None,
                up: None,
                hash: hash.clone(),
                tx: Some(tx),
                highlight: false,
            })),
            up: None,
            hash,
            tx: None,
            highlight: false,
        }
    }

    fn add_hash(&mut self, new_hash: &String) {
        let mut s = DefaultHasher::new();
        self.hash.hash(&mut s);
        new_hash.hash(&mut s);
        self.hash = base62::encode(s.finish());
    }

    fn add_tx(&mut self, tx: Tx, position: usize, height: u32) -> ShortHash {
        if height == 1 {
            let hash = tx.new_hash();
            let leaf = Box::new(Node {
                down: None,
                up: None,
                hash: hash.clone(),
                tx: Some(tx),
                highlight: false,
            });
            if self.down.is_none() {
                self.down = Some(leaf);
                self.hash = hash.clone();
                return hash;
            }
            self.add_hash(&hash);
            self.up = Some(leaf);
            return self.hash.clone();
        }
        if self.down.is_none() {
            let mut new_node = Box::new(Node::new());
            let hash = new_node.add_tx(tx, position, height - 1);
            self.down = Some(new_node);
            self.hash = hash.clone();
            return hash;
        }
        let half = 2usize.pow(height - 1);
        if position < half {
            let hash = self.down.as_mut().unwrap().add_tx(tx, position, height - 1);
            self.hash = hash.clone();
            return hash;
        }
        if self.up.is_none() {
            let mut new_node = Box::new(Node::new());
            let hash = new_node.add_tx(tx, position - half, height - 1);
            self.up = Some(new_node);
            self.add_hash(&hash);
            return self.hash.clone();
        }
        if position >= half {
            let hash = self
                .up
                .as_mut()
                .unwrap()
                .add_tx(tx, position - half, height - 1);
            self.add_hash(&hash);
            return self.hash.clone();
        }
        unreachable!("Somehow a case was never handled.");
    }

    fn clear_highlights(&mut self) {
        if self.highlight {
            self.highlight = false;
        }
        if let Some(down) = self.down.as_mut() {
            down.clear_highlights();
        }
        if let Some(up) = self.up.as_mut() {
            up.clear_highlights();
        }
    }

    fn highlight_tx(&mut self, tx_id: usize, range: (usize, usize)) {
        if let Some(tx) = &self.tx {
            if tx.id == tx_id {
                self.highlight = true;
                return;
            }
        }
        let half = range.0 + ((range.1 - range.0) / 2);
        if tx_id >= half {
            if let Some(down) = self.down.as_mut() {
                down.highlight = true;
            }
            self.up
                .as_mut()
                .unwrap()
                .highlight_tx(tx_id, (half, range.1));
        } else {
            if let Some(up) = self.up.as_mut() {
                up.highlight = true;
            }
            self.down
                .as_mut()
                .unwrap()
                .highlight_tx(tx_id, (range.0, half));
        }
    }

    fn leaf_to_html(&self, last_tx: usize, ctx: &Context<MerkleTree>) -> Html {
        let tx = self.tx.as_ref().unwrap();
        let id = tx.id;
        let color = match (last_tx - tx.id, self.highlight) {
            (_, highlight) if { highlight } => "has-text-success",
            (index, _) if { index == 0 } => "has-text-danger",
            (index, _) if { index == 1 } => "has-text-warning",
            _ => "has-text-info",
        };
        let classes = vec![
            "button",
            "is-small",
            "is-responsive",
            "is-light",
            "is-family-monospace",
            color,
        ];
        html! {
            <button class={classes!(classes)}
                onclick={ctx.link().callback(move |_| Msg::HighlightTx(id))}>
                {&self.hash}{" (#"}{tx.id}{"): "}{&tx.from}{" -> "}{&tx.to}{": $"}{&tx.amount}
            </button>
        }
    }

    fn branch_to_html(&self, color_index: usize, last_tx: usize, ctx: &Context<MerkleTree>) -> Html {
        let style = format!(
            "border-radius: 5px; margin: 2px; {}",
            get_color(color_index)
        );
        let classes = vec![
            "column",
            "label",
            "is-small",
            "is-family-monospace",
            "has-text-centered",
        ];
        let highlight = match &self.highlight {
            true => "has-text-success",
            false => "has-text-link",
        };
        html! {
            <div class="columns is-gapless hover-border is-vcentered is-mobile" style={style}>
               <div class={classes!(classes, highlight)}>
                   {&self.hash}
               </div>
               if self.down.is_some() || self.up.is_some() {
                   <div class="column">
                   if let Some(up) = &self.up {
                       {up.to_html(color_index + 2, last_tx, ctx)}
                   }
                   if let Some(down) = &self.down {
                       {down.to_html(color_index + 1, last_tx, ctx)}
                   }
                   </div>
               }
            </div>
        }
    }

    fn to_html(&self, color_index: usize, last_tx: usize, ctx: &Context<MerkleTree>) -> Html {
        match &self.tx.is_some() {
            true => self.leaf_to_html(last_tx, ctx),
            false => self.branch_to_html(color_index, last_tx, ctx),
        }
    }
}

pub struct MerkleTree {
    total_tx: usize,
    new_tx: NewTx,
    root: Node,
}

impl Component for MerkleTree {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        MerkleTree {
            total_tx: 0,
            new_tx: NewTx::new(),
            root: Node::new(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AddTx => {
                let to = self.new_tx.to.cast::<HtmlInputElement>().unwrap().value();
                let from = self.new_tx.from.cast::<HtmlInputElement>().unwrap().value();
                let amount = self
                    .new_tx
                    .amount
                    .cast::<HtmlInputElement>()
                    .unwrap()
                    .value()
                    .parse()
                    .unwrap_or(0);
                let tx = Tx {
                    to,
                    from,
                    amount,
                    id: self.total_tx,
                };
                self.add_tx(tx);
                true
            }
            Msg::HighlightTx(tx_id) => {
                self.highlight_tx(tx_id);
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
        <div class="section">
            <div class="columns box">
                <div class="column is-4 label has-text-centered">
                    {"Add transaction to the Merkle Tree:"}
                </div>
                <div class="column is-2"><input class="input is-info is-small"
                    value="from_X" type="text" ref={self.new_tx.from.clone()}/></div>
                <div class="column is-2"><input class="input is-warning is-small"
                    value="to_Y" type="text" ref={self.new_tx.to.clone()}/></div>
                <div class="column is-2"><input class="input is-danger is-small"
                    value="5" type="number" ref={self.new_tx.amount.clone()}/></div>
                <div class="column is-2"><button class="button is-success is-small is-fullwidth"
                    onclick={ctx.link().callback(|_| Msg::AddTx)}> {"Add Tx!"}</button></div>
            </div>
            <div>
                {self.render_tree(ctx)}
            </div>
        </div>
        }
    }
}

impl MerkleTree {
    fn last_tx(&self) -> Option<usize> {
        if self.total_tx == 0 {
            return None;
        }
        Some(self.total_tx - 1)
    }

    fn height(&self) -> u32 {
        let last_tx = match self.last_tx() {
            None => return 0,
            Some(tx) => tx,
        };
        let height = (last_tx as f32).log2() as u32;
        height + 1
    }

    fn new_root(&mut self) {
        let mut new_root = Node::new();
        new_root.hash = self.root.hash.clone();
        let old_root = std::mem::replace(&mut self.root, new_root);
        self.root.down = Some(Box::new(old_root));
    }

    fn highlight_tx(&mut self, tx_id: usize) {
        self.root.clear_highlights();
        self.root.highlight = true;
        self.root
            .highlight_tx(tx_id, (0, 2usize.pow(self.height())));
    }

    fn add_tx(&mut self, tx: Tx) {
        let position = self.total_tx;
        self.total_tx += 1;
        let height = self.height();
        if self.total_tx == 1 {
            self.root = Node::bootstrap(tx);
            return;
        }
        if position > 1 && position.is_power_of_two() {
            self.new_root();
        }
        self.root.add_tx(tx, position, height);
    }

    fn render_tree(&self, ctx: &Context<Self>) -> Html {
        if self.total_tx == 0 {
            return html! {
            <div class="section has-text-centered">
                <h1 class="subtitle"> {"The tree will appear here."} </h1>
            </div>
            };
        }
        html! {
        <div>
            <div class="section">

                <div class="has-text-centered">
                    {"The tree has "}<strong>{self.total_tx}</strong>{" transcations."}
                    {"The root hash is "}
                    <strong class="is-family-monospace">{&self.root.hash}</strong>{"."}
                </div>

                if self.total_tx > 2 {
                    <div class="has-text-centered is-size-7">
                        {"Hover over a group of transactions to isolate their hash. "}
                        {"Click on a transaction to highlight the hashes required to verify it."}
                    </div>
                }
                {self.root.to_html(0, self.last_tx().unwrap_or(0), ctx)}
            </div>
            <div class="has-text-centered">
                <a href="https://github.com/Gheo-Tech/yew-merkle-tree" class="href">
                    {"Check the source code."}
                </a>
            </div>
        </div> }
    }
}

fn main() {
    yew::start_app::<MerkleTree>();
}
