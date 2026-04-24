module a #(parameter int WIDTH = 8, parameter int DEPTH = 16) (input wire clk, output logic [WIDTH-1:0] q);
  localparam int MAX = WIDTH * DEPTH;
  always_ff @(posedge clk) q <= q + 1;
endmodule
