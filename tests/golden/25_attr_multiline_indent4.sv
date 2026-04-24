// config: indent_width=4
(* Something,
foo, bar *)
module m;
endmodule
// expected -----
(*
    Something,
    foo, bar
*)
module m;
endmodule
