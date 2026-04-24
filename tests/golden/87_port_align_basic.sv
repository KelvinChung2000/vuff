module m(
    output            mem_la_read,
    output            mem_la_write,
    output [31:0] mem_la_addr,
output reg [31:0] mem_la_wdata,
output reg [3:0] mem_la_wstrb
);
endmodule
// expected -----
module m (
  output            mem_la_read,
  output            mem_la_write,
  output     [31:0] mem_la_addr,
  output reg [31:0] mem_la_wdata,
  output reg [ 3:0] mem_la_wstrb
);
endmodule
