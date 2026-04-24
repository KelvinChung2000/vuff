module m;
  logic [7:0] mem [0:255];
  logic [7:0] a;
  assign a = mem [ 10 ];
endmodule
// expected -----
module m;
  logic [7:0] mem [0:255];
  logic [7:0] a;
  assign a = mem[10];
endmodule
