(* dont_touch = "true" *)
module tb_top;
  logic clk = 0;
  always #5 clk = ~clk;

  (* keep = "true" *)
  logic rst_n = 0;

  initial begin
    #20 rst_n = 1;
    #200 $finish;
  end

  (* mark_debug = "true" *)
  dut u_dut (.clk(clk), .rst_n(rst_n));
endmodule
// expected -----
(* dont_touch = "true" *)
module tb_top;
  logic clk = 0;
  always #5 clk = ~clk;

  (* keep = "true" *)
  logic rst_n = 0;

  initial begin
    #20 rst_n = 1;
    #200 $finish;
  end

  (* mark_debug = "true" *)
  dut u_dut (.clk(clk), .rst_n(rst_n));
endmodule
