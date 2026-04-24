module m;
  logic [7:0] a;
  assign a = { 3 { x } };
  assign a = { 4 { b, c } };
endmodule
// expected -----
module m;
  logic [7:0] a;
  assign a = {3{x}};
  assign a = {4{b, c}};
endmodule
