module m;
  wire  [7:0]   d_in   =  8'h00;
  logic [3:0]   nib    =  4'b1010;
  int   counter  =  0;
endmodule
// expected -----
module m;
  wire [7:0] d_in = 8'h00;
  logic [3:0] nib = 4'b1010;
  int counter = 0;
endmodule
