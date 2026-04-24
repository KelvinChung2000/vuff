module a (input wire clk, input wire rst_n, output logic q);
  always_ff @(posedge clk or negedge rst_n) begin
    if (!rst_n) q <= 1'b0;
    else q <= ~q;
  end
endmodule
