module m;
  logic [15:0] a;
  assign a = { {8{1'b0}}, data };
endmodule
// expected -----
module m;
  logic [15:0] a;
  assign a = {{8{1'b0}}, data};
endmodule
