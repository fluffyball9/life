use rustc_hash::FxBuildHasher;
use std::cell::Cell;
use std::collections::HashMap;
use std::mem::{self, MaybeUninit};
use std::rc::Rc;
use wasm_bindgen::prelude::wasm_bindgen;

#[global_allocator]
static A: rlsf::GlobalTlsf = rlsf::GlobalTlsf::new();

const INITIAL_CAPACITY: usize = 10_000;
const MASK_LEFT: usize = 1;
const MASK_TOP: usize = 2;
const MASK_RIGHT: usize = 4;
const MASK_BOTTOM: usize = 8;

//static mut COLLISION_COUNT: i32 = 0;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn time(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn timeEnd(s: &str);
}

#[repr(C)]
struct TreeNodeMaybeUninit {
    nw: MaybeUninit<Rc<TreeNodeMaybeUninit>>,
    ne: MaybeUninit<Rc<TreeNodeMaybeUninit>>,
    sw: MaybeUninit<Rc<TreeNodeMaybeUninit>>,
    se: MaybeUninit<Rc<TreeNodeMaybeUninit>>,
    population: usize,
    level: usize,
    cache: Cell<Option<Rc<TreeNodeMaybeUninit>>>,
    quick_cache: Cell<Option<Rc<TreeNodeMaybeUninit>>>,
    in_tree: Cell<bool>,
}

#[repr(C)]
struct TreeNode {
    nw: Rc<TreeNode>,
    ne: Rc<TreeNode>,
    sw: Rc<TreeNode>,
    se: Rc<TreeNode>,
    population: usize,
    level: usize,
    cache: Cell<Option<Rc<TreeNode>>>,
    quick_cache: Cell<Option<Rc<TreeNode>>>,
    in_tree: Cell<bool>,
}

impl PartialEq for TreeNode {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self, other)
    }
}

impl Eq for TreeNode {}

impl TreeNode {
    pub fn new(
        nw: Rc<TreeNode>,
        ne: Rc<TreeNode>,
        sw: Rc<TreeNode>,
        se: Rc<TreeNode>,
    ) -> Rc<TreeNode> {
        let mut new_node = Self {
            nw: nw,
            ne: ne,
            sw: sw,
            se: se,
            population: 0,
            level: 0,
            cache: Cell::new(None),
            quick_cache: Cell::new(None),
            in_tree: Cell::new(true),
        };
        new_node.level = new_node.nw.level + 1;
        // log(format!("Creating new node with level: {} and id: {}", new_node.level, id).as_str());
        new_node.population = new_node.nw.population
            + new_node.ne.population
            + new_node.sw.population
            + new_node.se.population;
        Rc::new(new_node)
    }

    pub fn new_leaf(population: usize) -> Rc<TreeNode> {
        // log("Creating new leaf...");
        let new_leaf = Rc::into_raw(Rc::new(TreeNodeMaybeUninit {
            nw: MaybeUninit::uninit(),
            ne: MaybeUninit::uninit(),
            sw: MaybeUninit::uninit(),
            se: MaybeUninit::uninit(),
            population: population,
            level: 0,
            cache: Cell::new(None),
            quick_cache: Cell::new(None),
            in_tree: Cell::new(true),
        })) as *mut TreeNodeMaybeUninit;
        let new_leaf = unsafe {
            (*new_leaf).nw = MaybeUninit::new(Rc::from_raw(new_leaf));
            (*new_leaf).ne = MaybeUninit::new(Rc::from_raw(new_leaf));
            (*new_leaf).sw = MaybeUninit::new(Rc::from_raw(new_leaf));
            (*new_leaf).se = MaybeUninit::new(Rc::from_raw(new_leaf));

            mem::transmute::<_, Rc<TreeNode>>(Rc::from_raw(new_leaf))
        };
        debug_assert!(new_leaf.get_cache() == None, "Cache is not None");
        debug_assert!(
            new_leaf.get_quick_cache() == None,
            "Quick cache is not None"
        );
        debug_assert_eq!(new_leaf.population, population);
        debug_assert_eq!(new_leaf.level, 0);
        new_leaf
    }

    pub fn get_cache(&self) -> Option<Rc<TreeNode>> {
        let cached = self.cache.take();
        let ret = cached.clone();
        self.cache.set(cached);
        ret
    }

    pub fn get_quick_cache(&self) -> Option<Rc<TreeNode>> {
        let cached = self.quick_cache.take();
        let ret = cached.clone();
        self.quick_cache.set(cached);
        ret
    }
}

struct Bounds {
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}

#[wasm_bindgen]
struct LifeUniverse {
    hashmap: HashMap<[usize; 4], Rc<TreeNode>, FxBuildHasher>,
    empty_tree_cache: Vec<Rc<TreeNode>>,
    level2_cache: Vec<Option<Rc<TreeNode>>>,
    rule_b: usize,
    rule_s: usize,
    root: Rc<TreeNode>,
    rewind_state: Option<Rc<TreeNode>>,
    step: usize,
    generation: f64,
    false_leaf: Rc<TreeNode>,
    true_leaf: Rc<TreeNode>,
}

#[wasm_bindgen]
impl LifeUniverse {
    fn get_key(
        nw: &Rc<TreeNode>,
        ne: &Rc<TreeNode>,
        sw: &Rc<TreeNode>,
        se: &Rc<TreeNode>,
    ) -> [usize; 4] {
        [
            Rc::as_ptr(nw) as usize,
            Rc::as_ptr(ne) as usize,
            Rc::as_ptr(sw) as usize,
            Rc::as_ptr(se) as usize,
        ]
    }

    fn mark_node(node: &Rc<TreeNode>, in_tree: bool) {
        if node.in_tree.get() != in_tree {
            node.in_tree.set(in_tree);
            if node.level > 1 {
                Self::mark_node(&node.nw, in_tree);
                Self::mark_node(&node.ne, in_tree);
                Self::mark_node(&node.sw, in_tree);
                Self::mark_node(&node.se, in_tree);

                if let Some(cached) = node.get_cache() {
                    Self::mark_node(&cached, in_tree);
                }

                if let Some(cached) = node.get_quick_cache() {
                    Self::mark_node(&cached, in_tree);
                }
            }
        }
    }

    fn reset_caches(&mut self) {
        self.empty_tree_cache.clear();
        self.level2_cache.fill(None);
    }

    fn garbage_collect(
        hashmap: &mut HashMap<[usize; 4], Rc<TreeNode>, FxBuildHasher>,
        root: &Rc<TreeNode>,
    ) {
        // log(format!("Garbage collecting..., current hs_size: {}, last_id: {}", self.hashmap_size, self.last_id).as_str());
        // time("GC: reset hashmap");

        Self::mark_node(&root, true);
        hashmap.retain(|_, v| v.in_tree.get()); // caches are one level lower so no memory leak
        Self::mark_node(&root, false); // reset mark

        // resize if over half full
        hashmap.reserve(
            hashmap
                .len()
                .saturating_mul(2)
                .saturating_sub(hashmap.capacity()),
        );
        // timeEnd("GC: reset hashmap");

        // log(format!("Garbage collection done..., new hs_size: {}, last_id: {}", self.hashmap_size, self.last_id).as_str());
    }

    fn create_tree(
        hashmap: &mut HashMap<[usize; 4], Rc<TreeNode>, FxBuildHasher>,
        root: &Rc<TreeNode>,
        nw: &Rc<TreeNode>,
        ne: &Rc<TreeNode>,
        sw: &Rc<TreeNode>,
        se: &Rc<TreeNode>,
    ) -> Rc<TreeNode> {
        debug_assert_eq!(nw.level, ne.level);
        debug_assert_eq!(nw.level, sw.level);
        debug_assert_eq!(nw.level, se.level);

        if hashmap.len() == hashmap.capacity() {
            Self::garbage_collect(hashmap, root);
        }

        hashmap
            .entry(Self::get_key(nw, ne, sw, se))
            .or_insert_with(|| TreeNode::new(nw.clone(), ne.clone(), sw.clone(), se.clone()))
            .clone()
    }

    fn empty_tree<'a>(
        empty_tree_cache: &'a mut Vec<Rc<TreeNode>>,
        false_leaf: &Rc<TreeNode>,
        hashmap: &mut HashMap<[usize; 4], Rc<TreeNode>, FxBuildHasher>,
        root: &Rc<TreeNode>,
        level: usize,
    ) -> &'a Rc<TreeNode> {
        for _ in empty_tree_cache.len()..=level {
                if let Some(last) = empty_tree_cache.last() {
                let new_node = Self::create_tree(hashmap, &root, last, last, last, last);
                empty_tree_cache.push(new_node);
            } else {
                empty_tree_cache.push(false_leaf.clone());
            }
        }
        &empty_tree_cache[level]
    }

    #[allow(dead_code)]
    pub fn clear_pattern(&mut self) {
        self.hashmap = HashMap::with_capacity_and_hasher(INITIAL_CAPACITY, Default::default());
        self.empty_tree_cache.clear();
        self.level2_cache = vec![None; 0x10000];
        self.root = Self::empty_tree(&mut self.empty_tree_cache, &self.false_leaf, &mut self.hashmap, &self.root, 3).clone();
        self.generation = 0.0;
        // log("Clearing pattern...");
    }

    #[wasm_bindgen(constructor)]
    #[allow(dead_code)]
    pub fn new() -> LifeUniverse {
        // log("Starting constructor...");
        // log("Creating object...");
        let false_leaf = TreeNode::new_leaf(0);
        let true_leaf = TreeNode::new_leaf(1);
        let mut ret = LifeUniverse {
            hashmap: HashMap::default(),
            empty_tree_cache: vec![],
            level2_cache: vec![],
            root: true_leaf.clone(),
            generation: 0.0,
            rule_b: 1 << 3,
            rule_s: 1 << 2 | 1 << 3,
            rewind_state: None,
            step: 0,
            false_leaf: false_leaf,
            true_leaf: true_leaf,
        };
        // log("Clearing pattern...");
        ret.clear_pattern();
        // log("Done clearing patter...");
        ret
    }

    fn pow2(x: usize) -> f64 {
        2_f64.powi(x.try_into().unwrap_or(i32::MAX))
    }

    #[allow(dead_code)]
    pub fn save_rewind_state(&mut self) {
        self.rewind_state = Some(self.root.clone());
    }

    #[allow(dead_code)]
    pub fn restore_rewind_state(&mut self) {
        if let Some(rewind_state) = &self.rewind_state {
            self.generation = 0.0;
            self.root = rewind_state.clone();
            Self::garbage_collect(&mut self.hashmap, &self.root);
        }
    }

    #[allow(dead_code)]
    pub fn has_rewind_state(&self) -> bool {
        self.rewind_state.is_some()
    }

    fn eval_mask(&self, mask: usize) -> usize {
        let rule = if mask & 32 != 0 {
            self.rule_s
        } else {
            self.rule_b
        };

        rule >> (mask & 0x757).count_ones() & 1
    }

    fn level1_create(&mut self, mask: usize) -> Rc<TreeNode> {
        let true_leaf = self.true_leaf.clone();
        let false_leaf = self.false_leaf.clone();
        Self::create_tree(
            &mut self.hashmap,
            &self.root,
            if mask & 1 != 0 {
                &true_leaf
            } else {
                &false_leaf
            },
            if mask & 2 != 0 {
                &true_leaf
            } else {
                &false_leaf
            },
            if mask & 4 != 0 {
                &true_leaf
            } else {
                &false_leaf
            },
            if mask & 8 != 0 {
                &true_leaf
            } else {
                &false_leaf
            },
        )
    }

    fn get_level_from_bounds(&self, bounds: Vec<f64>) -> usize {
        let mut max = 4.0;

        for coordinate in bounds {
            if coordinate + 1.0 > max {
                max = coordinate + 1.0;
            } else if -coordinate > max {
                max = -coordinate;
            }
        }

        max.log2().ceil() as usize + 1
    }

    fn node_set_bit(&mut self, node: &Rc<TreeNode>, x: f64, y: f64, living: bool) -> Rc<TreeNode> {
        if node.level == 0 {
            return if living {
                self.true_leaf.clone()
            } else {
                self.false_leaf.clone()
            };
        }

        let offset = if node.level == 1 {
            0.0
        } else {
            Self::pow2(node.level - 2)
        };

        let changed: Rc<TreeNode>;
        let mut nw = &node.nw;
        let mut ne = &node.ne;
        let mut sw = &node.sw;
        let mut se = &node.se;

        if x < 0.0 {
            if y < 0.0 {
                changed = self.node_set_bit(&nw, x + offset, y + offset, living);
                nw = &changed;
            } else {
                changed = self.node_set_bit(&sw, x + offset, y - offset, living);
                sw = &changed;
            }
        } else {
            if y < 0.0 {
                changed = self.node_set_bit(&ne, x - offset, y + offset, living);
                ne = &changed;
            } else {
                changed = self.node_set_bit(&se, x - offset, y - offset, living);
                se = &changed;
            }
        }

        Self::create_tree(&mut self.hashmap, &self.root, nw, ne, sw, se)
    }

    fn node_get_bit(&self, node: &Rc<TreeNode>, x: f64, y: f64) -> bool {
        if node.population == 0 {
            return false;
        }
        if node.level == 0 {
            // other level 0 case handled above
            return true;
        }

        let offset = if node.level == 1 {
            0.0
        } else {
            Self::pow2(node.level - 2)
        };

        if x < 0.0 {
            if y < 0.0 {
                self.node_get_bit(&node.nw, x + offset, y + offset)
            } else {
                self.node_get_bit(&node.sw, x + offset, y - offset)
            }
        } else {
            if y < 0.0 {
                self.node_get_bit(&node.ne, x - offset, y + offset)
            } else {
                self.node_get_bit(&node.se, x - offset, y - offset)
            }
        }
    }

    #[allow(dead_code)]
    pub fn set_bit(&mut self, x: f64, y: f64, living: bool) {
        // log(format!("Setting bit at x: {}, y: {}, living: {}", x, y, living).as_str());
        let level = self.get_level_from_bounds(vec![x, y]);

        if living {
            while level > self.root.level {
                self.root = self.expand_universe(self.root.clone());
            }
        } else if level > self.root.level {
            // no need to delete pixels outside of the universe
            return;
        }

        self.root = self.node_set_bit(&self.root.clone(), x, y, living);
    }

    #[allow(dead_code)]
    pub fn get_bit(&self, x: f64, y: f64) -> bool {
        let level = self.get_level_from_bounds(vec![x, y]);

        if level > self.root.level {
            return false;
        }

        self.node_get_bit(&self.root, x, y)
    }

    fn node_get_boundary(
        &self,
        node: &Rc<TreeNode>,
        left: f64,
        top: f64,
        find_mask: usize,
        boundary: &mut Vec<f64>,
    ) {
        if node.population == 0 || find_mask == 0 {
            return;
        }

        if node.level == 0 {
            if left < boundary[0] {
                boundary[0] = left;
            }
            if left > boundary[1] {
                boundary[1] = left;
            }

            if top < boundary[2] {
                boundary[2] = top;
            }
            if top > boundary[3] {
                boundary[3] = top;
            }
        } else {
            let offset = Self::pow2(node.level - 1);

            if left >= boundary[0]
                && left + offset * 2.0 <= boundary[1]
                && top >= boundary[2]
                && top + offset * 2.0 <= boundary[3]
            {
                // this square is already inside the found boundary
                return;
            }

            let mut find_nw = find_mask;
            let mut find_ne = find_mask;
            let mut find_sw = find_mask;
            let mut find_se = find_mask;

            if node.nw.population != 0 {
                find_sw &= !MASK_TOP;
                find_ne &= !MASK_LEFT;
                find_se &= !MASK_TOP & !MASK_LEFT;
            }
            if node.sw.population != 0 {
                find_se &= !MASK_LEFT;
                find_nw &= !MASK_BOTTOM;
                find_ne &= !MASK_BOTTOM & !MASK_LEFT;
            }
            if node.ne.population != 0 {
                find_nw &= !MASK_RIGHT;
                find_se &= !MASK_TOP;
                find_sw &= !MASK_TOP & !MASK_RIGHT;
            }
            if node.se.population != 0 {
                find_sw &= !MASK_RIGHT;
                find_ne &= !MASK_BOTTOM;
                find_nw &= !MASK_BOTTOM & !MASK_RIGHT;
            }

            self.node_get_boundary(&node.nw, left, top, find_nw, boundary);
            self.node_get_boundary(&node.sw, left, top + offset, find_sw, boundary);
            self.node_get_boundary(&node.ne, left + offset, top, find_ne, boundary);
            self.node_get_boundary(&node.se, left + offset, top + offset, find_se, boundary);
        }
    }

    #[allow(dead_code)]
    pub fn get_root_bounds(&self) -> Vec<f64> {
        if self.root.population == 0 {
            return vec![0.0, 0.0, 0.0, 0.0];
        }

        let mut bounds = vec![
            f64::INFINITY,     // left
            f64::NEG_INFINITY, // right
            f64::INFINITY,     // top
            f64::NEG_INFINITY, // bottom
        ];
        let offset = Self::pow2(self.root.level - 1);

        self.node_get_boundary(
            &self.root,
            -offset,
            -offset,
            MASK_LEFT | MASK_TOP | MASK_RIGHT | MASK_BOTTOM,
            &mut bounds,
        );

        bounds
    }

    fn expand_universe(&mut self, node: Rc<TreeNode>) -> Rc<TreeNode> {
        let level = node.level;
        let hashmap = &mut self.hashmap;
        let t = Self::empty_tree(&mut self.empty_tree_cache, &self.false_leaf, hashmap, &self.root, level - 1);
        let nw = Self::create_tree(hashmap, &self.root, &t, &t, &t, &node.nw);
        let ne = Self::create_tree(hashmap, &self.root, &t, &t, &node.ne, &t);
        let sw = Self::create_tree(hashmap, &self.root, &t, &node.sw, &t, &t);
        let se = Self::create_tree(hashmap, &self.root, &node.se, &t, &t, &t);

        Self::create_tree(&mut self.hashmap, &self.root, &nw, &ne, &sw, &se)
    }

    fn uncache(&mut self, also_quick: bool) {
        for (_, n) in &mut self.hashmap {
            n.cache.take();
            if also_quick {
                n.quick_cache.take();
            }
        }
    }

    fn node_level2_next(&mut self, node: &Rc<TreeNode>) -> Rc<TreeNode> {
        let nw = &node.nw;
        let ne = &node.ne;
        let sw = &node.sw;
        let se = &node.se;
        let bitmask = nw.nw.population << 15
            | nw.ne.population << 14
            | ne.nw.population << 13
            | ne.ne.population << 12
            | nw.sw.population << 11
            | nw.se.population << 10
            | ne.sw.population << 9
            | ne.se.population << 8
            | sw.nw.population << 7
            | sw.ne.population << 6
            | se.nw.population << 5
            | se.ne.population << 4
            | sw.sw.population << 3
            | sw.se.population << 2
            | se.sw.population << 1
            | se.se.population;

        self.level1_create(
            self.eval_mask(bitmask >> 5)
                | self.eval_mask(bitmask >> 4) << 1
                | self.eval_mask(bitmask >> 1) << 2
                | self.eval_mask(bitmask) << 3,
        )
    }

    fn node_quick_next_generation(&mut self, node: &Rc<TreeNode>) -> Rc<TreeNode> {
        if let Some(cached) = node.get_quick_cache() {
            debug_assert_eq!(cached.level, node.level - 1);
            return cached;
        }

        if node.level == 2 {
            let new_node = self.node_level2_next(&node);
            node.quick_cache.set(Some(new_node.clone()));
            return new_node;
        }

        let nw = &node.nw;
        let ne = &node.ne;
        let sw = &node.sw;
        let se = &node.se;
        let n00 = self.node_quick_next_generation(&nw);
        let n01_tree = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &nw.ne,
            &ne.nw,
            &nw.se,
            &ne.sw,
        );
        let n01 = self.node_quick_next_generation(&n01_tree);
        let n02 = self.node_quick_next_generation(&ne);
        let n10_tree = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &nw.sw,
            &nw.se,
            &sw.nw,
            &sw.ne,
        );
        let n10 = self.node_quick_next_generation(&n10_tree);
        let n11_tree = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &nw.se,
            &ne.sw,
            &sw.ne,
            &se.nw,
        );
        let n11 = self.node_quick_next_generation(&n11_tree);
        let n12_tree = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &ne.sw,
            &ne.se,
            &se.nw,
            &se.ne,
        );
        let n12 = self.node_quick_next_generation(&n12_tree);
        let n20 = self.node_quick_next_generation(&sw);
        let n21_tree = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &sw.ne,
            &se.nw,
            &sw.se,
            &se.sw,
        );
        let n21 = self.node_quick_next_generation(&n21_tree);
        let n22 = self.node_quick_next_generation(&se);

        let n00_n01_n10_n11 =
            Self::create_tree(&mut self.hashmap, &self.root, &n00, &n01, &n10, &n11);
        let n01_n02_n11_n12 =
            Self::create_tree(&mut self.hashmap, &self.root, &n01, &n02, &n11, &n12);
        let n10_n11_n20_n21 =
            Self::create_tree(&mut self.hashmap, &self.root, &n10, &n11, &n20, &n21);
        let n11_n12_n21_n22 =
            Self::create_tree(&mut self.hashmap, &self.root, &n11, &n12, &n21, &n22);

        let nw_tree = self.node_quick_next_generation(&n00_n01_n10_n11);
        let ne_tree = self.node_quick_next_generation(&n01_n02_n11_n12);
        let sw_tree = self.node_quick_next_generation(&n10_n11_n20_n21);
        let se_tree = self.node_quick_next_generation(&n11_n12_n21_n22);

        let new_node = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &nw_tree,
            &ne_tree,
            &sw_tree,
            &se_tree,
        );

        debug_assert_eq!(new_node.level, node.level - 1);
        node.quick_cache.set(Some(new_node.clone()));
        new_node
    }

    fn node_next_generation(&mut self, node: &Rc<TreeNode>) -> Rc<TreeNode> {
        if let Some(cached) = node.get_cache() {
            return cached;
        }

        if self.step == node.level - 2 {
            return self.node_quick_next_generation(&node);
        }

        if node.level == 2 {
            if let Some(cached) = node.get_quick_cache() {
                return cached;
            } else {
                let new_node = self.node_level2_next(&node);
                node.quick_cache.set(Some(new_node.clone()));
                return new_node;
            }
        }

        let nw = &node.nw;
        let ne = &node.ne;
        let sw = &node.sw;
        let se = &node.se;
        let n00 = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &nw.nw.se,
            &nw.ne.sw,
            &nw.sw.ne,
            &nw.se.nw,
        );
        let n01 = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &nw.ne.se,
            &ne.nw.sw,
            &nw.se.ne,
            &ne.sw.nw,
        );
        let n02 = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &ne.nw.se,
            &ne.ne.sw,
            &ne.sw.ne,
            &ne.se.nw,
        );
        let n10 = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &nw.sw.se,
            &nw.se.sw,
            &sw.nw.ne,
            &sw.ne.nw,
        );
        let n11 = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &nw.se.se,
            &ne.sw.sw,
            &sw.ne.ne,
            &se.nw.nw,
        );
        let n12 = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &ne.sw.se,
            &ne.se.sw,
            &se.nw.ne,
            &se.ne.nw,
        );
        let n20 = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &sw.nw.se,
            &sw.ne.sw,
            &sw.sw.ne,
            &sw.se.nw,
        );
        let n21 = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &sw.ne.se,
            &se.nw.sw,
            &sw.se.ne,
            &se.sw.nw,
        );
        let n22 = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &se.nw.se,
            &se.ne.sw,
            &se.sw.ne,
            &se.se.nw,
        );

        //debug_assert!(n00.level == n01.level && n00.level == n02.level && n00.level == n10.level && n00.level == n11.level && n00.level == n12.level && n00.level == n20.level && n00.level == n21.level && n00.level == n22.level);

        let n00_n01_n10_n11 =
            Self::create_tree(&mut self.hashmap, &self.root, &n00, &n01, &n10, &n11);
        let n01_n02_n11_n12 =
            Self::create_tree(&mut self.hashmap, &self.root, &n01, &n02, &n11, &n12);
        let n10_n11_n20_n21 =
            Self::create_tree(&mut self.hashmap, &self.root, &n10, &n11, &n20, &n21);
        let n11_n12_n21_n22 =
            Self::create_tree(&mut self.hashmap, &self.root, &n11, &n12, &n21, &n22);

        let nw_tree = self.node_next_generation(&n00_n01_n10_n11);
        let ne_tree = self.node_next_generation(&n01_n02_n11_n12);
        let sw_tree = self.node_next_generation(&n10_n11_n20_n21);
        let se_tree = self.node_next_generation(&n11_n12_n21_n22);

        let new_node = Self::create_tree(
            &mut self.hashmap,
            &self.root,
            &nw_tree,
            &ne_tree,
            &sw_tree,
            &se_tree,
        );

        node.cache.set(Some(new_node.clone()));
        new_node
    }

    #[allow(dead_code)]
    pub fn next_generation(&mut self, is_single: bool) {
        /*unsafe {
            COLLISION_COUNT = 0;
        }*/
        let mut root = self.root.clone();

        while (is_single && root.level <= self.step + 2)
            || root.nw.population != root.nw.se.se.population
            || root.ne.population != root.ne.sw.sw.population
            || root.sw.population != root.sw.ne.ne.population
            || root.se.population != root.se.nw.nw.population
        {
            root = self.expand_universe(root);
        }

        // superstep button doesn't exist
        /*if is_single {
            self.generation += Self::pow2(self.step);
            root = self.node_next_generation(root);
        } else {
            self.generation += Self::pow2(self.root.level - 2);
            root = self.node_quick_next_generation(root);
        }*/
        self.generation += Self::pow2(self.step);
        root = self.node_next_generation(&root);

        // log(format!("Collision count: {}", unsafe { COLLISION_COUNT }).as_str());

        self.root = root;
    }

    fn get_bounds(&self, field_x: &Vec<i32>, field_y: &Vec<i32>) -> Bounds {
        if field_x.is_empty() {
            return Bounds {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
            };
        }

        let mut bounds = Bounds {
            left: field_x[0],
            right: field_x[0],
            top: field_y[0],
            bottom: field_y[0],
        };

        for i in 1..field_x.len() {
            if field_x[i] < bounds.left {
                bounds.left = field_x[i];
            } else if field_x[i] > bounds.right {
                bounds.right = field_x[i];
            }

            if field_y[i] < bounds.top {
                bounds.top = field_y[i];
            } else if field_y[i] > bounds.bottom {
                bounds.bottom = field_y[i];
            }
        }

        bounds
    }

    fn move_field(
        &mut self,
        field_x: &mut Vec<i32>,
        field_y: &mut Vec<i32>,
        offset_x: i32,
        offset_y: i32,
    ) {
        for i in 0..field_x.len() {
            field_x[i] += offset_x;
            field_y[i] += offset_y;
        }
    }

    fn partition(
        &self,
        start: usize,
        end: usize,
        test_field: &mut Vec<i32>,
        other_field: &mut Vec<i32>,
        offset: i32,
    ) -> usize {
        let mut i = start;
        let mut j = end;

        while j != usize::MAX && i <= j {
            while i <= end && (test_field[i] & offset == 0) {
                i += 1;
            }

            while j > start && (test_field[j] & offset != 0) {
                // no need to check for out of bounds since min start is 0 and j > start
                j -= 1;
            }

            if i >= j {
                break;
            }

            test_field.swap(i, j);
            other_field.swap(i, j);

            i += 1;
            j = j.wrapping_sub(1);
        }

        i
    }

    fn level2_setup(
        &mut self,
        start: usize,
        end: usize,
        field_x: &mut Vec<i32>,
        field_y: &mut Vec<i32>,
    ) -> Rc<TreeNode> {
        let mut set = 0;
        // log("Start level2_setup");
        for i in start..=end {
            let x = field_x[i];
            let y = field_y[i];

            // log(format!("x: {}, y: {}", x, y).as_str());
            set |= 1 << (x & 1 | (y & 1 | x & 2) << 1 | (y & 2) << 2);
        }

        if let Some(cached) = &self.level2_cache[set] {
            cached.clone()
        } else {
            let nw = self.level1_create(set);
            let ne = self.level1_create(set >> 4);
            let sw = self.level1_create(set >> 8);
            let se = self.level1_create(set >> 12);

            let new_node = Self::create_tree(&mut self.hashmap, &self.root, &nw, &ne, &sw, &se);

            self.level2_cache[set].insert(new_node).clone()
        }
    }

    fn setup_field_recurse(
        &mut self,
        start: usize,
        end: usize,
        field_x: &mut Vec<i32>,
        field_y: &mut Vec<i32>,
        mut level: usize,
    ) -> Rc<TreeNode> {
        // log(format!("From recurse: current level is: {}", level).as_str());
        if start > end || end == usize::MAX
        /* wrapped around */
        {
            return Self::empty_tree(&mut self.empty_tree_cache, &self.false_leaf, &mut self.hashmap, &self.root, level).clone();
        }

        if level == 2 {
            return self.level2_setup(start, end, field_x, field_y);
        }

        level -= 1;

        // log("From recurse: partitioning...");

        let offset = 1 << level;
        let part3 = self.partition(start, end, field_y, field_x, offset);
        let part2 = self.partition(start, part3.wrapping_sub(1), field_x, field_y, offset);
        let part4 = self.partition(part3, end, field_x, field_y, offset);

        // log("From recurse: recursing...");

        let nw = self.setup_field_recurse(start, part2.wrapping_sub(1), field_x, field_y, level);
        let ne = self.setup_field_recurse(part2, part3.wrapping_sub(1), field_x, field_y, level);
        let sw = self.setup_field_recurse(part3, part4.wrapping_sub(1), field_x, field_y, level);
        let se = self.setup_field_recurse(part4, end, field_x, field_y, level);

        // log("From recurse: creating tree...");

        Self::create_tree(&mut self.hashmap, &self.root, &nw, &ne, &sw, &se)
    }

    #[allow(dead_code)]
    pub fn setup_field(&mut self, mut field_x: Vec<i32>, mut field_y: Vec<i32>) {
        debug_assert_eq!(field_x.len(), field_y.len());
        let mut bounds = self.get_bounds(&field_x, &field_y);
        let offset_x = ((bounds.left - bounds.right + 1) / 2) - bounds.left;
        let offset_y = ((bounds.top - bounds.bottom + 1) / 2) - bounds.top;

        self.move_field(&mut field_x, &mut field_y, offset_x as i32, offset_y as i32);

        bounds.left += offset_x;
        bounds.right += offset_x;
        bounds.top += offset_y;
        bounds.bottom += offset_y;

        let level = self
            .get_level_from_bounds(vec![
                bounds.left as f64,
                bounds.right as f64,
                bounds.top as f64,
                bounds.bottom as f64,
            ])
            .max(4);
        let offset = 1 << (level - 1) as i32;
        let count = field_x.len();

        self.move_field(&mut field_x, &mut field_y, offset, offset);

        self.root = self.setup_field_recurse(0, count - 1, &mut field_x, &mut field_y, level);
    }

    #[allow(dead_code)]
    pub fn get_step(&self) -> usize {
        self.step
    }

    #[allow(dead_code)]
    pub fn set_step(&mut self, step: usize) {
        if step != self.step {
            self.step = step;

            self.uncache(false);
            self.reset_caches();
        }
    }

    #[allow(dead_code)]
    pub fn set_rules(&mut self, s: usize, b: usize) {
        if self.rule_s != s || self.rule_b != b {
            self.rule_s = s;
            self.rule_b = b;

            self.uncache(true);
            self.reset_caches();
        }
    }

    #[allow(dead_code)]
    pub fn get_rule_s(&self) -> usize {
        self.rule_s
    }

    #[allow(dead_code)]
    pub fn get_rule_b(&self) -> usize {
        self.rule_b
    }

    fn draw_node(
        node: &Rc<TreeNode>,
        data: &mut Vec<f64>,
        x: f64,
        y: f64,
        size: f64,
        offset_x: f64,
        offset_y: f64,
        height: f64,
        width: f64,
    ) {
        // log(format!("Drawing node... Population: {}, Level: {}", node.population, node.level).as_str());
        if node.population == 0
            || x + size + offset_x < 0.0
            || y + size + offset_y < 0.0
            || x + offset_x >= width
            || y + offset_y >= height
        {
            // don't draw outside of screen
            return;
        }

        if size <= 1.0 || node.level == 0 {
            // no need to check if population is 0, because we already did that earlier
            data.push(x + offset_x);
            data.push(y + offset_y);
        } else {
            let size = size / 2.0;

            Self::draw_node(
                &node.nw, data, x, y, size, offset_x, offset_y, height, width,
            );
            Self::draw_node(
                &node.ne,
                data,
                x + size,
                y,
                size,
                offset_x,
                offset_y,
                height,
                width,
            );
            Self::draw_node(
                &node.sw,
                data,
                x,
                y + size,
                size,
                offset_x,
                offset_y,
                height,
                width,
            );
            Self::draw_node(
                &node.se,
                data,
                x + size,
                y + size,
                size,
                offset_x,
                offset_y,
                height,
                width,
            );
        }
    }

    #[allow(dead_code)]
    pub fn draw(
        &self,
        x: f64,
        y: f64,
        size: f64,
        height: f64,
        width: f64,
        offset_x: f64,
        offset_y: f64,
    ) -> Vec<f64> {
        let mut data = Vec::new();
        // log(format!("Starting draw with: x: {}, y: {}, size: {}, offset_x: {}, offset_y: {}, height: {}, width: {}", x, y, size, offset_x, offset_y, height, width).as_str());
        Self::draw_node(
            &self.root, &mut data, x, y, size, offset_x, offset_y, height, width,
        );
        data
    }

    #[allow(dead_code)]
    pub fn get_generation(&self) -> f64 {
        self.generation
    }

    #[allow(dead_code)]
    pub fn get_population(&self) -> usize {
        self.root.population
    }

    #[allow(dead_code)]
    pub fn get_level(&self) -> usize {
        self.root.level
    }
}
