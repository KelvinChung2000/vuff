module m;
  wire   [7:0]   bus;
  logic  [31:0]  word;
  reg    [3:0]   nib;
endmodule
// expected -----
module m;
  wire [7:0] bus;
  logic [31:0] word;
  reg [3:0] nib;
endmodule
