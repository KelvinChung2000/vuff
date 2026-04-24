module m;
  logic [7:0] bus;
  assign q = sel ? bus[7:4] : bus[3:0];
endmodule
// expected -----
module m;
  logic [7:0] bus;
  assign q = sel ? bus[7:4] : bus[3:0];
endmodule
