module m;
  logic [3:0] a;
  initial a = '{ 1, 2, 3, 4 };
endmodule
// expected -----
module m;
  logic [3:0] a;
  initial a = '{1, 2, 3, 4};
endmodule
