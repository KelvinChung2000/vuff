(*mark_debug="true",keep="yes"*)
logic foo;
module m;
(*dont_touch="true"*)
logic bar;
endmodule
// expected -----
(* mark_debug = "true", keep = "yes" *)
logic foo;
module m;
  (* dont_touch = "true" *)
  logic bar;
endmodule
