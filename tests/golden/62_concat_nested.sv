module m;
  logic [7:0] a;
  assign a = { b, { c, d }, e };
endmodule
// expected -----
module m;
  logic [7:0] a;
  assign a = {b, {c, d}, e};
endmodule
