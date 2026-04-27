module m;
  logic   q [0:3];
  logic   r [4];
  logic   [7:0]  mem  [256];
  logic   [7:0]  mem2 [0:255];
endmodule
// expected -----
module m;
  logic q [0:3];
  logic r [4];
  logic [7:0] mem [256];
  logic [7:0] mem2 [0:255];
endmodule
