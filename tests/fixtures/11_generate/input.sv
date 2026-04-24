module a #(parameter int N = 4) (input wire clk, output logic [N-1:0] q);
  genvar i;
  generate
    for (i = 0; i < N; i++) begin : g
      always_ff @(posedge clk) q[i] <= ~q[i];
    end
  endgenerate
endmodule
