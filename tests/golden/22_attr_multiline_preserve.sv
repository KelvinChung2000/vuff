(* Something, foo,
bar,
  ffo,
bbaaa,
last_attr *)
module m;
  assign x = 1;
endmodule
// expected -----
(*
  Something, foo,
  bar,
  ffo,
  bbaaa,
  last_attr
*)
module m;
  assign x = 1;
endmodule
