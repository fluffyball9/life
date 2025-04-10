use rustc_hash::FxBuildHasher;
use std::cell::Cell;
use std::collections::HashMap;
use std::mem::{self, MaybeUninit};
use std::rc::Rc;
use wasm_bindgen::prelude::wasm_bindgen;

const INITIAL_SIZE: usize = 15;
const HASHMAP_LIMIT: usize = 22;
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
    hashmap_size: usize,
    hashmap: HashMap<[usize; 4], Rc<TreeNode>, FxBuildHasher>,
    empty_tree_cache: Vec<Option<Rc<TreeNode>>>,
    level2_cache: Vec<Option<Rc<TreeNode>>>,
    rule_b: usize,
    rule_s: usize,
    root: Rc<TreeNode>,
    rewind_state: Option<Rc<TreeNode>>,
    step: usize,
    generation: f64,
    false_leaf: Rc<TreeNode>,
    true_leaf: Rc<TreeNode>,
    _powers: Vec<f64>,
}

#[wasm_bindgen]
impl LifeUniverse {
    fn get_key2(
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

    fn get_key(n: &Rc<TreeNode>) -> [usize; 4] {
        Self::get_key2(&n.nw, &n.ne, &n.sw, &n.se)
    }

    fn in_hashmap(&self, n: &Rc<TreeNode>) -> bool {
        self.hashmap.contains_key(&Self::get_key(n))
    }

    fn node_hash(&mut self, node: Rc<TreeNode>) {
        if !self.in_hashmap(&node) {
            if node.level > 1 {
                self.node_hash(node.nw.clone());
                self.node_hash(node.ne.clone());
                self.node_hash(node.sw.clone());
                self.node_hash(node.se.clone());

                if let Some(cached) = node.get_cache() {
                    self.node_hash(cached);
                }

                if let Some(cached) = node.get_quick_cache() {
                    self.node_hash(cached);
                }
            }

            self.hashmap_insert(node);
        }
    }

    fn reset_caches(&mut self) {
        self.empty_tree_cache.fill(None);
        self.level2_cache.fill(None);
    }

    fn garbage_collect(&mut self) {
        // log(format!("Garbage collecting..., current hs_size: {}, last_id: {}", self.hashmap_size, self.last_id).as_str());
        // time("GC: reset hashmap");

        self.hashmap.clear();
        if self.hashmap_size < (1 << HASHMAP_LIMIT) - 1 {
            self.hashmap_size = self.hashmap_size << 1 | 1;
        }
        self.hashmap
            .reserve(self.hashmap_size.saturating_sub(self.hashmap.capacity()));
        // timeEnd("GC: reset hashmap");

        // time("GC: rehashing nodes");
        self.node_hash(self.root.clone());
        // timeEnd("GC: rehashing nodes");

        // log(format!("Garbage collection done..., new hs_size: {}, last_id: {}", self.hashmap_size, self.last_id).as_str());
    }

    fn create_tree(
        &mut self,
        nw: Rc<TreeNode>,
        ne: Rc<TreeNode>,
        sw: Rc<TreeNode>,
        se: Rc<TreeNode>,
    ) -> Rc<TreeNode> {
        debug_assert_eq!(nw.level, ne.level);
        debug_assert_eq!(nw.level, sw.level);
        debug_assert_eq!(nw.level, se.level);

        if self.hashmap.len() == self.hashmap_size {
            self.garbage_collect();
            return self.create_tree(nw, ne, sw, se);
        }

        self.hashmap
            .entry(Self::get_key2(&nw, &ne, &sw, &se))
            .or_insert_with(|| TreeNode::new(nw, ne, sw, se))
            .clone()
    }

    fn empty_tree(&mut self, level: usize) -> Rc<TreeNode> {
        if self.empty_tree_cache.len() <= level {
            self.empty_tree_cache.resize(level + 1, None);
        } else if let Some(cached) = &self.empty_tree_cache[level] {
            return cached.clone();
        }

        let t = if level == 1 {
            self.false_leaf.clone()
        } else {
            self.empty_tree(level - 1)
        };

        // log(format!("Level of t is: {}", t.level).as_str());

        let empty_tree = self.create_tree(t.clone(), t.clone(), t.clone(), t);

        // log(format!("Empty tree requested level of: {}", level).as_str());
        // log(format!("Empty tree created level of: {}", empty_tree.level).as_str());

        self.empty_tree_cache[level].insert(empty_tree).clone()
    }

    #[allow(dead_code)]
    pub fn clear_pattern(&mut self) {
        self.hashmap_size = (1 << INITIAL_SIZE) - 1;
        self.hashmap = HashMap::with_capacity_and_hasher(self.hashmap_size, Default::default());
        self.empty_tree_cache.fill(None);
        self.level2_cache = vec![None; 0x10000];
        self.root = self.empty_tree(3);
        self.generation = 0.0;
        // log("Clearing pattern...");
    }

    #[wasm_bindgen(constructor)]
    #[allow(dead_code)]
    pub fn new() -> LifeUniverse {
        // log("Starting constructor...");
        let mut powers = vec![0.0; 1024];
        powers[0] = 1.0;
        for i in 1..1024 {
            powers[i] = powers[i - 1] * 2.0;
        }
        // log("Creating object...");
        let false_leaf = TreeNode::new_leaf(0);
        let true_leaf = TreeNode::new_leaf(1);
        let mut ret = LifeUniverse {
            hashmap_size: 0,
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
            _powers: powers,
        };
        // log("Clearing pattern...");
        ret.clear_pattern();
        // log("Done clearing patter...");
        ret
    }

    fn pow2(&self, x: usize) -> f64 {
        if x >= 1024 {
            return f64::INFINITY;
        }
        self._powers[x as usize]
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

            self.garbage_collect();
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
        self.create_tree(
            if mask & 1 != 0 {
                self.true_leaf.clone()
            } else {
                self.false_leaf.clone()
            },
            if mask & 2 != 0 {
                self.true_leaf.clone()
            } else {
                self.false_leaf.clone()
            },
            if mask & 4 != 0 {
                self.true_leaf.clone()
            } else {
                self.false_leaf.clone()
            },
            if mask & 8 != 0 {
                self.true_leaf.clone()
            } else {
                self.false_leaf.clone()
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

    fn node_set_bit(&mut self, node: Rc<TreeNode>, x: f64, y: f64, living: bool) -> Rc<TreeNode> {
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
            self.pow2(node.level - 2)
        };
        let mut nw = node.nw.clone();
        let mut ne = node.ne.clone();
        let mut sw = node.sw.clone();
        let mut se = node.se.clone();

        if x < 0.0 {
            if y < 0.0 {
                nw = self.node_set_bit(nw, x + offset, y + offset, living);
            } else {
                sw = self.node_set_bit(sw, x + offset, y - offset, living);
            }
        } else {
            if y < 0.0 {
                ne = self.node_set_bit(ne, x - offset, y + offset, living);
            } else {
                se = self.node_set_bit(se, x - offset, y - offset, living);
            }
        }

        self.create_tree(nw, ne, sw, se)
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
            self.pow2(node.level - 2)
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

        self.root = self.node_set_bit(self.root.clone(), x, y, living);
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
            let offset = self.pow2(node.level - 1);

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
        let offset = self.pow2(self.root.level - 1);

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
        let t = self.empty_tree(level - 1);
        let nw = self.create_tree(t.clone(), t.clone(), t.clone(), node.nw.clone());
        let ne = self.create_tree(t.clone(), t.clone(), node.ne.clone(), t.clone());
        let sw = self.create_tree(t.clone(), node.sw.clone(), t.clone(), t.clone());
        let se = self.create_tree(node.se.clone(), t.clone(), t.clone(), t.clone());

        self.create_tree(nw, ne, sw, se)
    }

    fn uncache(&mut self, also_quick: bool) {
        for (_, n) in &mut self.hashmap {
            n.cache.take();
            if also_quick {
                n.quick_cache.take();
            }
        }
    }

    fn hashmap_insert(&mut self, n: Rc<TreeNode>) {
        self.hashmap.insert(Self::get_key(&n), n);
    }

    fn node_level2_next(&mut self, node: Rc<TreeNode>) -> Rc<TreeNode> {
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

    fn node_quick_next_generation(&mut self, node: Rc<TreeNode>) -> Rc<TreeNode> {
        if let Some(cached) = node.get_quick_cache() {
            debug_assert_eq!(cached.level, node.level - 1);
            return cached;
        }

        if node.level == 2 {
            let new_node = self.node_level2_next(node.clone());
            node.quick_cache.set(Some(new_node.clone()));
            return new_node;
        }

        let nw = &node.nw;
        let ne = &node.ne;
        let sw = &node.sw;
        let se = &node.se;
        let n00 = self.node_quick_next_generation(nw.clone());
        let n01_tree = self.create_tree(nw.ne.clone(), ne.nw.clone(), nw.se.clone(), ne.sw.clone());
        let n01 = self.node_quick_next_generation(n01_tree);
        let n02 = self.node_quick_next_generation(ne.clone());
        let n10_tree = self.create_tree(nw.sw.clone(), nw.se.clone(), sw.nw.clone(), sw.ne.clone());
        let n10 = self.node_quick_next_generation(n10_tree);
        let n11_tree = self.create_tree(nw.se.clone(), ne.sw.clone(), sw.ne.clone(), se.nw.clone());
        let n11 = self.node_quick_next_generation(n11_tree);
        let n12_tree = self.create_tree(ne.sw.clone(), ne.se.clone(), se.nw.clone(), se.ne.clone());
        let n12 = self.node_quick_next_generation(n12_tree);
        let n20 = self.node_quick_next_generation(sw.clone());
        let n21_tree = self.create_tree(sw.ne.clone(), se.nw.clone(), sw.se.clone(), se.sw.clone());
        let n21 = self.node_quick_next_generation(n21_tree);
        let n22 = self.node_quick_next_generation(se.clone());

        let n00_n01_n10_n11 = self.create_tree(n00, n01.clone(), n10.clone(), n11.clone());
        let n01_n02_n11_n12 = self.create_tree(n01, n02, n11.clone(), n12.clone());
        let n10_n11_n20_n21 = self.create_tree(n10, n11.clone(), n20, n21.clone());
        let n11_n12_n21_n22 = self.create_tree(n11, n12, n21, n22);

        let nw_tree = self.node_quick_next_generation(n00_n01_n10_n11);
        let ne_tree = self.node_quick_next_generation(n01_n02_n11_n12);
        let sw_tree = self.node_quick_next_generation(n10_n11_n20_n21);
        let se_tree = self.node_quick_next_generation(n11_n12_n21_n22);

        let new_node = self.create_tree(nw_tree, ne_tree, sw_tree, se_tree);

        debug_assert_eq!(new_node.level, node.level - 1);
        node.quick_cache.set(Some(new_node.clone()));
        new_node
    }

    fn node_next_generation(&mut self, node: Rc<TreeNode>) -> Rc<TreeNode> {
        if let Some(cached) = node.get_cache() {
            return cached;
        }

        if self.step == node.level - 2 {
            return self.node_quick_next_generation(node);
        }

        if node.level == 2 {
            if let Some(cached) = node.get_quick_cache() {
                return cached;
            } else {
                let new_node = self.node_level2_next(node.clone());
                node.quick_cache.set(Some(new_node.clone()));
                return new_node;
            }
        }

        let nw = &node.nw;
        let ne = &node.ne;
        let sw = &node.sw;
        let se = &node.se;
        let n00 = self.create_tree(
            nw.nw.se.clone(),
            nw.ne.sw.clone(),
            nw.sw.ne.clone(),
            nw.se.nw.clone(),
        );
        let n01 = self.create_tree(
            nw.ne.se.clone(),
            ne.nw.sw.clone(),
            nw.se.ne.clone(),
            ne.sw.nw.clone(),
        );
        let n02 = self.create_tree(
            ne.nw.se.clone(),
            ne.ne.sw.clone(),
            ne.sw.ne.clone(),
            ne.se.nw.clone(),
        );
        let n10 = self.create_tree(
            nw.sw.se.clone(),
            nw.se.sw.clone(),
            sw.nw.ne.clone(),
            sw.ne.nw.clone(),
        );
        let n11 = self.create_tree(
            nw.se.se.clone(),
            ne.sw.sw.clone(),
            sw.ne.ne.clone(),
            se.nw.nw.clone(),
        );
        let n12 = self.create_tree(
            ne.sw.se.clone(),
            ne.se.sw.clone(),
            se.nw.ne.clone(),
            se.ne.nw.clone(),
        );
        let n20 = self.create_tree(
            sw.nw.se.clone(),
            sw.ne.sw.clone(),
            sw.sw.ne.clone(),
            sw.se.nw.clone(),
        );
        let n21 = self.create_tree(
            sw.ne.se.clone(),
            se.nw.sw.clone(),
            sw.se.ne.clone(),
            se.sw.nw.clone(),
        );
        let n22 = self.create_tree(
            se.nw.se.clone(),
            se.ne.sw.clone(),
            se.sw.ne.clone(),
            se.se.nw.clone(),
        );

        //debug_assert!(n00.level == n01.level && n00.level == n02.level && n00.level == n10.level && n00.level == n11.level && n00.level == n12.level && n00.level == n20.level && n00.level == n21.level && n00.level == n22.level);

        let n00_n01_n10_n11 = self.create_tree(n00, n01.clone(), n10.clone(), n11.clone());
        let n01_n02_n11_n12 = self.create_tree(n01, n02, n11.clone(), n12.clone());
        let n10_n11_n20_n21 = self.create_tree(n10, n11.clone(), n20, n21.clone());
        let n11_n12_n21_n22 = self.create_tree(n11, n12, n21, n22);

        let nw_tree = self.node_next_generation(n00_n01_n10_n11);
        let ne_tree = self.node_next_generation(n01_n02_n11_n12);
        let sw_tree = self.node_next_generation(n10_n11_n20_n21);
        let se_tree = self.node_next_generation(n11_n12_n21_n22);

        let new_node = self.create_tree(nw_tree, ne_tree, sw_tree, se_tree);

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
            self.generation += self.pow2(self.step);
            root = self.node_next_generation(root);
        } else {
            self.generation += self.pow2(self.root.level - 2);
            root = self.node_quick_next_generation(root);
        }*/
        self.generation += self.pow2(self.step);
        root = self.node_next_generation(root);

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

            let new_node = self.create_tree(nw, ne, sw, se);

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
            return self.empty_tree(level);
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

        self.create_tree(nw, ne, sw, se)
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
