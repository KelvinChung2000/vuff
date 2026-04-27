module m;
  parameter   int   W   =  8;
  localparam  int   D   =  16;
  parameter   [7:0] P   =  8'h0F;
endmodule
// expected -----
module m;
  parameter int W = 8;
  localparam int D = 16;
  parameter [7:0] P = 8'h0F;
endmodule
