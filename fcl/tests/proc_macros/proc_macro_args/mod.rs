


// params
// closure_coords
// prefix (low priority)

// outer
//   |
//   V      inner -> | mod | trait | impl | struct | impl trait | fn             | static
// ------------------------------------------------------------------------      ----------------
// mod               |     |       |      |        |            | both           |       |      mod/fn
// trait             |     |       |      |        |            |                |       |
// impl struct       |     |       |      |        |            |                |       | 
// impl trait        |     |       |      |        |            |                |       |
// fn                |     |       |      |        |            | params         |       |      fn/fn
//                   |     |       |      |        |            | closure_coords |       |
// static            |     |       |      |        |            |                |       |


mod fn_fn_call_params;
mod fn_fn_closure_coords;
mod mod_fn;
// fn_mod
